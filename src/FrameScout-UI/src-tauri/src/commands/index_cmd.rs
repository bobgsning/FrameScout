// 负责文件与文件夹扫描、批量调用 AI 向量化并写入数据库与内存矩阵。

use std::sync::atomic::Ordering;
use std::time::Instant;

use rusqlite::params;
use serde_json::json;
use tauri::{AppHandle, Emitter, State};
use walkdir::WalkDir;

use crate::constants::{IMAGE_EXTENSIONS, VIDEO_EXTENSIONS, RPC_TIMEOUT_INDEX_BASE_MS};
use crate::fs_trace;
use crate::model_code::{AppState, ProgressPayload};
use crate::proto::framescout as proto;
use crate::services::request_vector;
use crate::storage::journal;
use crate::storage::normalize_path;
use crate::storage::text_store::TextVectorRow;
use crate::storage::vector_blob::encode_f32_le;
use crate::time_util::utc_stamp;

pub async fn process_file_paths_internal(
    app: &AppHandle,
    state: &State<'_, AppState>,
    pending_files: Vec<String>,
    enable_ocr: bool,
    ocr_languages: Vec<String>,
) -> Result<usize, String> {
    let total_to_process = pending_files.len();

    if total_to_process == 0 {
        let _ = app.emit(
            "scan-progress",
            ProgressPayload {
                status: "✅ Done".to_string(),
                file_path: "No new files to process".to_string(),
                current: 0,
                total: 0,
                new_files: vec![],
            },
        );
        return Ok(0);
    }

    if enable_ocr && ocr_languages.is_empty() {
        return Err("OCR enabled but no languages specified".to_string());
    }

    let mut consecutive_failures = 0;
    const MAX_CONSECUTIVE_FAILURES: usize = 5;

    // 按目录分布：让用户一眼看出这次扫描的素材来自哪些文件夹。
    // 只统计前 5 个目录，避免 summary 被长尾淹没。
    let mut dir_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for p in &pending_files {
        let cut = p.rfind('\\').or_else(|| p.rfind('/'));
        let dir = match cut {
            Some(i) => p[..i].to_string(),
            None => p.clone(),
        };
        *dir_counts.entry(dir).or_insert(0) += 1;
    }
    let mut top_dirs: Vec<(String, usize)> = dir_counts.into_iter().collect();
    top_dirs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    top_dirs.truncate(5);

    // 本次扫描/索引的批次骨架。batch_id 沿用 `scan_<UTC>` 命名约定，
    // 会同时写进 `batches` 表与每行的 `frame_vectors.indexed_in_batch_id`，
    // 于是「按批次重编向量 / 模型升级回溯 / 按批次清理」直接查该列，少一跳 join。
    // 批次写不进去不应阻断扫描，故失败时退化为「只有 id、没有骨架」。
    let batch_id = {
        let db = state.db_conn.lock().unwrap();
        match journal::begin_batch(&db, journal::BatchType::Scan, None) {
            Ok(id) => id,
            Err(e) => {
                println!("⚠️ Failed to open a scan batch (continuing without skeleton): {}", e);
                format!("scan_{}", utc_stamp())
            }
        }
    };

    let scan_started = Instant::now();
    let mut added_count: usize = 0;
    let mut first_seen_count = 0usize;
    let mut changed_count = 0usize;
    let mut failed_paths: Vec<String> = Vec::new();
    let mut current_idx = 0;
    let mut batch_seq = 0usize;
    let mut cancelled = false;
    let batch_size = 4;

    for chunk in pending_files.chunks(batch_size) {
        batch_seq += 1;
        let batch_started = Instant::now();

        // 取消检查（P0-6 / 第三轮 B13）：每批开头检查 cancel_flag。
        // cancel_scan 命令把它置 true，本循环即停止后续批处理并返回已完成数量。
        // 旧实现无取消机制，点错盘符只能杀进程。
        if state.cancel_flag.load(Ordering::Relaxed) {
            let _ = app.emit(
                "scan-progress",
                ProgressPayload {
                    status: "🛑 Cancelled by user".to_string(),
                    file_path: "Scan cancelled, partial results saved".to_string(),
                    current: current_idx,
                    total: total_to_process,
                    new_files: vec![],
                },
            );
            cancelled = true;
            break;
        }

        // 检查试用限额
        {
            let memory = state.memory_db.read().map_err(|e| e.to_string())?;
            if let Err(err_msg) = state.trial_guard.lock().unwrap().check_limit(memory.len()) {
                let _ = app.emit(
                    "scan-progress",
                    ProgressPayload {
                        status: "🛑 Trial Limit Reached".to_string(),
                        file_path: err_msg.clone(),
                        current: current_idx,
                        total: total_to_process,
                        new_files: vec![],
                    },
                );
                return Err(err_msg);
            }
        }
        current_idx += chunk.len();

        let batch_payload = proto::BatchTask {
            file_paths: chunk.to_vec(),
            ocr_config: Some(proto::OcrConfig {
                enable_ocr: Some(enable_ocr),
                languages: ocr_languages.clone(),
            }),
        };
        let payload = proto::encode_request::Payload::Batch(batch_payload);

        match request_vector(payload, RPC_TIMEOUT_INDEX_BASE_MS) {
            Ok(frames) => {
                consecutive_failures = 0;

                let mut db = state.db_conn.lock().unwrap();
                let mut memory = state.memory_db.write().map_err(|e| e.to_string())?;

                let tx_result = db.transaction();
                let mut batch_added_files = Vec::new();

                if let Ok(tx) = tx_result {
                    // ① 感知：为每个待处理路径记录观察状态与生命事件。
                    //    先于「行动」落账——即使后面推理失败，files 里也留了锚点，
                    //    该文件会进入差异报告第四组「需要索引但无向量」等待重试，
                    //    而不会因为 observed_* 已写就被静默永久跳过。
                    for pending_path in chunk {
                        let stat = journal::stat_file(pending_path);
                        match journal::upsert_file_observed(&tx, pending_path, stat) {
                            Ok(journal::Observation::FirstSeen) => {
                                let _ = journal::record_event(
                                    &tx,
                                    &batch_id,
                                    pending_path,
                                    journal::EventType::FirstSeen,
                                    None,
                                );
                                first_seen_count += 1;
                            }
                            Ok(journal::Observation::Changed) => {
                                let _ = journal::record_event(
                                    &tx,
                                    &batch_id,
                                    pending_path,
                                    journal::EventType::ObservedChange,
                                    None,
                                );
                                changed_count += 1;
                            }
                            Ok(journal::Observation::Unchanged) => {}
                            Err(e) => println!("⚠️ Failed to record observation for {}: {}", pending_path, e),
                        }
                    }

                    // ② 行动：写入帧向量 + 行级 OCR + 文本向量。
                    //    先按 path 归拢该文件的所有帧，落库时「帧向量 + OCR 行级 + 文本向量」
                    //    以文件为单位整体覆盖，避免三者处于半截状态。
                    let mut ocr_by_path: std::collections::HashMap<String, Vec<crate::model_code::OcrLine>> =
                        std::collections::HashMap::new();
                    let mut text_by_path: std::collections::HashMap<String, Vec<TextVectorRow>> =
                        std::collections::HashMap::new();

                    for frame in frames {
                        let file_path = frame.path;
                        let timestamp = frame.timestamp;
                        let vec = frame.vector;
                        let ocr_text = frame.ocr_text;
                        let index_time = frame.index_time;

                        // 只写 vector_f32 BLOB，不再写遗留的 vector_json
                        //（旧格式是一次性消耗品，不为它支付双写成本）。
                        let vec_blob = encode_f32_le(&vec);
                        let insert_ok = tx.execute(
                            "INSERT INTO frame_vectors
                                (path, timestamp, vector_f32, indexed_in_batch_id, ocr_text, user_note, index_time)
                             VALUES (?1, ?2, ?3, ?4, ?5, '', ?6)
                             ON CONFLICT(path, timestamp) DO UPDATE SET
                             vector_f32 = excluded.vector_f32,
                             indexed_in_batch_id = excluded.indexed_in_batch_id,
                             ocr_text = excluded.ocr_text,
                             index_time = excluded.index_time",
                            params![
                                file_path.clone(),
                                timestamp,
                                vec_blob,
                                batch_id.clone(),
                                ocr_text.clone(),
                                index_time
                            ],
                        ).is_ok();
                        if insert_ok {
                            memory.push(file_path.clone(), timestamp, vec, ocr_text, "".to_string(), index_time);
                            batch_added_files.push(file_path.clone());
                            added_count += 1;
                        }

                        // 归拢文本向量（BGE-M3 dense），稍后以文件为单位覆盖写入。
                        // dense 非空即表示该帧 OCR 文字已抽成文本向量。
                        if !frame.dense_vector.is_empty() {
                            text_by_path.entry(file_path.clone()).or_default().push(TextVectorRow {
                                timestamp_ms: (timestamp as f64 * 1000.0).round() as i64,
                                dense: frame.dense_vector,
                            });
                        }

                        // 归拢行级 OCR，稍后以文件为单位覆盖写入
                        if !frame.ocr_lines.is_empty() {
                            ocr_by_path.entry(file_path).or_default().extend(frame.ocr_lines);
                        }
                    }

                    // ③ 落库行级 OCR 与文本向量（与帧向量同事务）。
                    //    **P0-5 修复**：旧实现这两个失败只 println! 但事务照常 commit，
                    //    导致「帧向量已落库但 OCR/文本向量丢失」的半截状态。
                    //    现在汇聚错误：任一失败记入 failed_paths 并 rollback 整批，
                    //    兑现注释承诺的「三者要么一起落库、要么一起回滚」。
                    let mut batch_store_errors: Vec<String> = Vec::new();
                    for (path, lines) in &ocr_by_path {
                        if let Err(e) = crate::storage::ocr_store::replace_ocr_entries(&tx, path, lines) {
                            println!("⚠️ Failed to store OCR entries for {}: {}", path, e);
                            batch_store_errors.push(path.clone());
                        }
                    }
                    for (path, rows) in &text_by_path {
                        if let Err(e) = crate::storage::text_store::replace_text_vectors(&tx, path, rows) {
                            println!("⚠️ Failed to store text vectors for {}: {}", path, e);
                            batch_store_errors.push(path.clone());
                        }
                    }

                    // **P0-5 修复**：tx.commit() 不再用 `let _ =` 吞错。
                    // 旧实现错误被静默丢弃，内存矩阵已 push 但磁盘未 commit ⇒
                    // 重启后结果静默消失。现在 commit 失败则 rollback 并记入 failed_paths。
                    if !batch_store_errors.is_empty() {
                        // OCR/text 写入失败：rollback 整批，所有路径记入失败清单。
                        // 债单 C2：rollback 分支漏回退内存矩阵——这批帧在 :220 已 push 过，
                        // 磁盘回滚了但内存还留着，正是 P0-5 想消灭的半截状态。补上 remove_by_paths。
                        let _ = tx.rollback();
                        memory.remove_by_paths(&batch_added_files);
                        failed_paths.extend(batch_store_errors.iter().cloned());
                        added_count = added_count.saturating_sub(batch_added_files.len());
                    } else {
                        match tx.commit() {
                            Ok(_) => {}
                            Err(e) => {
                                println!("⚠️ Failed to commit batch {}: {}", batch_seq, e);
                                // commit 失败意味着这批没落盘；内存里 push 的也回退。
                                let paths_in_batch: Vec<String> = batch_added_files.clone();
                                memory.remove_by_paths(&paths_in_batch);
                                added_count = added_count.saturating_sub(batch_added_files.len());
                                failed_paths.extend(chunk.iter().cloned());
                            }
                        }
                    }
                }

                fs_trace!(
                    "batch indexed, batch_id={}, seq={}, requested={}, inserted={}, took={:?}",
                    batch_id,
                    batch_seq,
                    chunk.len(),
                    batch_added_files.len(),
                    batch_started.elapsed()
                );

                let _ = app.emit(
                    "scan-progress",
                    ProgressPayload {
                        status: format!(
                            "🚀 Batch Processing ({}/{}), {} per group...",
                            current_idx, total_to_process, chunk.len()
                        ),
                        file_path: chunk.last().cloned().unwrap_or_default(),
                        current: current_idx,
                        total: total_to_process,
                        new_files: batch_added_files,
                    },
                );
            }
            Err(e) => {
                consecutive_failures += 1;
                // 整批失败：把该批路径记入扫描报告，供用户知晓「哪些没进去」。
                failed_paths.extend(chunk.iter().cloned());
                let _ = app.emit(
                    "scan-progress",
                    ProgressPayload {
                        status: format!("❌ Batch Failed: {}", e),
                        file_path: chunk.last().cloned().unwrap_or_default(),
                        current: current_idx,
                        total: total_to_process,
                        new_files: vec![],
                    },
                );
                if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                    let _ = app.emit(
                        "scan-progress",
                        ProgressPayload {
                            status: "🛑 Aborted: Too many consecutive failures".to_string(),
                            file_path: chunk.last().cloned().unwrap_or_default(),
                            current: current_idx,
                            total: total_to_process,
                            new_files: vec![],
                        },
                    );
                    return Err(format!("Aborted after {} consecutive failures: {}", consecutive_failures, e));
                }
            }
        }
    }

    // 债单 C5：取消后不再发 "✅ Done"（会把刚发的 "🛑 Cancelled" 覆盖掉）。
    let _ = app.emit(
        "scan-progress",
        ProgressPayload {
            status: if cancelled {
                "🛑 Cancelled".to_string()
            } else {
                "✅ Done".to_string()
            },
            file_path: if cancelled {
                "Scan cancelled, partial results saved".to_string()
            } else {
                "Queue Finished".to_string()
            },
            current: if cancelled { current_idx } else { total_to_process },
            total: total_to_process,
            new_files: vec![],
        },
    );

    let took = scan_started.elapsed();
    let took_ms = took.as_millis() as u64;

    // 扫描报告落 batches.summary_json（应用重启不丢，UI 读 WHERE batch_id = ? 即可）。
    // 这是活动中心的雏形，也是「让用户有掌控感」的那份清单。
    let summary = json!({
        "requested": total_to_process,
        "frames_inserted": added_count,
        "batches": batch_seq,
        "first_seen": first_seen_count,
        "observed_change": changed_count,
        "failed_count": failed_paths.len(),
        "failed_paths": failed_paths,
        "took_ms": took_ms,
        "avg_ms_per_file": if total_to_process > 0 { took_ms / total_to_process as u64 } else { 0 },
        "top_dirs": top_dirs.iter().map(|(dir, count)| json!({ "dir": dir, "count": count })).collect::<Vec<_>>(),
        "cancelled": cancelled,
    })
    .to_string();

    {
        let db = state.db_conn.lock().unwrap();
        // 债单 C5：被取消的批次用 cancelled 状态收尾，扫描报告据此区分「中止」与「完成」。
        let close_result = if cancelled {
            journal::finish_batch_cancelled(&db, &batch_id, &summary)
        } else {
            journal::finish_batch(&db, &batch_id, &summary)
        };
        if let Err(e) = close_result {
            println!("⚠️ Failed to close scan batch {}: {}", batch_id, e);
        }
    }

    fs_trace!(
        "scan finished, batch_id={}, requested={}, inserted={}, first_seen={}, changed={}, failed={}, batches={}, took={:?}",
        batch_id,
        total_to_process,
        added_count,
        first_seen_count,
        changed_count,
        failed_paths.len(),
        batch_seq,
        took
    );

    Ok(added_count)
}

#[tauri::command]
pub async fn scan_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    folder_path: String,
    scan_mode: String,
    enable_ocr: bool,
    ocr_languages: Vec<String>,
) -> Result<usize, String> {
    // 重置取消标志（P0-6）：上次扫描可能被取消留下 true，新扫描必须从 false 开始。
    state.cancel_flag.store(false, Ordering::Relaxed);

    {
        let memory = state.memory_db.read().map_err(|e| e.to_string())?;
        state.trial_guard.lock().unwrap().check_limit(memory.len())?;
    }

    let _ = app.emit(
        "scan-progress",
        ProgressPayload {
            status: "🔍 Rapid Pre-scanning...".to_string(),
            file_path: "".to_string(),
            current: 0,
            total: 0,
            new_files: vec![],
        },
    );

    let mut pending_files = Vec::new();

    for entry in WalkDir::new(&folder_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            let is_image = IMAGE_EXTENSIONS.contains(&ext.as_str());
            let is_video = VIDEO_EXTENSIONS.contains(&ext.as_str());
            let should_process = match scan_mode.as_str() {
                "image" => is_image,
                "video" => is_video,
                _ => is_image || is_video,
            };

            if should_process {
                // 落库前统一路径形态，避免同一文件因大小写/分隔符变体被记成两条。
                let path_str = normalize_path(&path.to_string_lossy());
                if path_str.is_empty() {
                    continue;
                }
                let exists = { state.memory_db.read().map_err(|e| e.to_string())?.contains_path(&path_str) };
                if !exists {
                    pending_files.push(path_str);
                }
            }
        }
    }

    if pending_files.is_empty() {
        let _ = app.emit(
            "scan-progress",
            ProgressPayload {
                status: "✅ Done".to_string(),
                file_path: "No new files found".to_string(),
                current: 0,
                total: 0,
                new_files: vec![],
            },
        );
        return Ok(0);
    }

    process_file_paths_internal(&app, &state, pending_files, enable_ocr, ocr_languages).await
}

#[tauri::command]
pub async fn index_files(
    app: AppHandle,
    state: State<'_, AppState>,
    file_paths: Vec<String>,
    enable_ocr: bool,
    ocr_languages: Vec<String>,
) -> Result<usize, String> {
    // 重置取消标志（P0-6）。
    state.cancel_flag.store(false, Ordering::Relaxed);

    {
        let memory = state.memory_db.read().map_err(|e| e.to_string())?;
        state.trial_guard.lock().unwrap().check_limit(memory.len())?;
    }
    let _ = app.emit(
        "scan-progress",
        ProgressPayload {
            status: format!("📁 Indexing {} files...", file_paths.len()),
            file_path: "".to_string(),
            current: 0,
            total: file_paths.len(),
            new_files: vec![],
        },
    );

    let mut pending_files = Vec::new();
    let total = file_paths.len();

    {
        let memory = state.memory_db.read().map_err(|e| e.to_string())?;
        for raw in &file_paths {
            // 落库前统一路径形态，与扫描路径保持同一规范。
            let path_str = normalize_path(raw);
            if path_str.is_empty() {
                continue;
            }
            if !memory.contains_path(&path_str) {
                pending_files.push(path_str);
            }
        }
    }

    if pending_files.is_empty() {
        let _ = app.emit(
            "scan-progress",
            ProgressPayload {
                status: "✅ Done".to_string(),
                file_path: "All files already indexed".to_string(),
                current: total,
                total: total,
                new_files: vec![],
            },
        );
        return Ok(0);
    }

    process_file_paths_internal(&app, &state, pending_files, enable_ocr, ocr_languages).await
}

/// 取消正在进行的扫描/索引（P0-6 / 第三轮 B13）。
///
/// 把 `AppState.cancel_flag` 置 true，`process_file_paths_internal` 的批循环
/// 会在下一批开头检测到并停止。已落库的批不回滚（partial-success 语义）。
#[tauri::command]
pub async fn cancel_scan(state: State<'_, AppState>) -> Result<bool, String> {
    state.cancel_flag.store(true, Ordering::Relaxed);
    Ok(true)
}