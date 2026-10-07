// 重索引：对「已索引但内容变了」的文件重新编码。
//
// 与 `index_files` 的区别（写死，别混淆）：
//   - `index_files` 只处理**库里没有**的文件，按路径跳过已索引项，写 first_seen 系列事件；
//   - 本命令**强制重新编码**已存在的文件，用于差异报告的「修改」组。
//
// 事件因果链（文档 5.2 定死，这里是最关键的一环）：
//   1. 扫描发现差异 → 写 observed_change（在扫描那条链路里）
//   2. 用户在差异报告确认「重新索引」 → **此刻不写任何事件**
//   3. 重索引成功完成 → 写 content_modified，payload 记 old/new frame count
//   4. 重索引失败 → 写 reindex_failed，文件继续留在差异报告第四组待重试
//
// **重索引与旧帧行处置（写死）**：成功 = 旧帧行被**事务内整体替换**——
// `DELETE FROM frame_vectors WHERE path = ?` 后插入新帧。
// 因此重索引**不可回退**：旧向量已被替换，撤销它无法恢复。
// 这与 `dead`（向量保留、可复活）是两套不同语义，切勿混为一谈。

use std::collections::HashMap;

use rusqlite::params;
use serde_json::json;
use tauri::State;

use crate::constants::RPC_TIMEOUT_INDEX_BASE_MS;
use crate::fs_trace;
use crate::model_code::{AppState, EncodedFrame, ReindexFailure, ReindexResult};
use crate::proto::framescout as proto;
use crate::services::request_vector;
use crate::storage::journal::{self, BatchType, EventType};
use crate::storage::vector_blob::encode_f32_le;

/// 与扫描一致的批大小（4 张/组）。
const BATCH_SIZE: usize = 4;

#[tauri::command]
pub async fn reindex_files(
    state: State<'_, AppState>,
    paths: Vec<String>,
    enable_ocr: bool,
    ocr_languages: Vec<String>,
) -> Result<ReindexResult, String> {
    if paths.is_empty() {
        return Err("No paths selected".to_string());
    }
    if enable_ocr && ocr_languages.is_empty() {
        return Err("OCR enabled but no languages specified".to_string());
    }

    // mut：重索引需要在事务里整体替换旧帧行
    let mut db = state.db_conn.lock().map_err(|e| e.to_string())?;
    let mut memory = state.memory_db.write().map_err(|e| e.to_string())?;

    let batch_id = journal::begin_batch(&db, BatchType::Reindex, None)
        .unwrap_or_else(|e| format!("reindex_fallback_{}", e));

    let mut succeeded: Vec<String> = Vec::new();
    let mut failed: Vec<ReindexFailure> = Vec::new();
    let mut replaced_frames = 0usize;
    let mut new_frames = 0usize;

    for chunk in paths.chunks(BATCH_SIZE) {
        let batch_payload = proto::BatchTask {
            file_paths: chunk.to_vec(),
            ocr_config: Some(proto::OcrConfig {
                enable_ocr: Some(enable_ocr),
                languages: ocr_languages.clone(),
            }),
        };

        match request_vector(proto::encode_request::Payload::Batch(batch_payload), RPC_TIMEOUT_INDEX_BASE_MS) {
            Ok(frames) => {
                // 按路径归组：一个视频会返回多帧，必须整批替换而不是逐帧覆盖。
                let mut by_path: HashMap<String, Vec<EncodedFrame>> = HashMap::new();
                for frame in frames {
                    by_path.entry(frame.path.clone()).or_default().push(frame);
                }

                for path in chunk {
                    let entries = match by_path.remove(path) {
                        Some(e) => e,
                        None => {
                            // worker 没返回这个文件的任何帧：记为失败，留在第四组
                            let _ = journal::record_event(
                                &db,
                                &batch_id,
                                path,
                                EventType::ReindexFailed,
                                Some(&json!({ "error": "worker returned no frames" }).to_string()),
                            );
                            failed.push(ReindexFailure {
                                path: path.clone(),
                                error: "worker returned no frames".to_string(),
                            });
                            continue;
                        }
                    };

                    let old_count: usize = db
                        .query_row(
                            "SELECT COUNT(*) FROM frame_vectors WHERE path = ?1",
                            params![path],
                            |row| row.get::<_, i64>(0),
                        )
                        .unwrap_or(0) as usize;
                    let inserted_count = entries.len();
                    // 文件级备注兜底：旧帧里任意一个非空备注都算数
                    let legacy_file_note = read_file_note(&db, path);

                    let tx = match db.transaction() {
                        Ok(tx) => tx,
                        Err(e) => {
                            failed.push(ReindexFailure {
                                path: path.clone(),
                                error: e.to_string(),
                            });
                            continue;
                        }
                    };

                    // 事务内整体替换：先备份旧帧 → 再删 → 再插新帧。
                    // 任何一步失败即回滚，绝不留下「旧的删了、新的没插上」的中间态。
                    let result = (|| -> Result<(), rusqlite::Error> {
                        // ① 冷备份：只能在 DELETE 之前做，与被删行同事务。
                        journal::snapshot_frames_before_replace(&tx, path, &batch_id)
                            .map_err(to_sql_error)?;

                        // ② 读出旧帧的用户数据：备注与已有 OCR 文本。
                        //    它们是用户资产，重索引只该换向量，不该顺手抹掉。
                        let legacy = read_legacy_frame_meta(&tx, path)?;

                        // ③ 删旧帧 + 删旧 OCR 行级 + 删旧文本向量（与帧向量同事务）
                        //    **P0-2 修复（第三轮 D5）**：旧实现只删 frame_vectors + media_ocr_entries，
                        //    不删 media_text_vectors ⇒ 重索引后文本搜索出现陈旧/重复命中。
                        tx.execute("DELETE FROM frame_vectors WHERE path = ?1", params![path])?;
                        tx.execute("DELETE FROM media_ocr_entries WHERE path = ?1", params![path])?;
                        tx.execute("DELETE FROM media_text_vectors WHERE path = ?1", params![path])?;

                        // ④ 插新帧：user_note 继承旧值；新帧若没带 OCR 文本
                        //    （例如本次没开 OCR），保留该帧原有的文本，不静默清空。
                        for frame in &entries {
                            let timestamp = frame.timestamp;
                            let vec = &frame.vector;
                            let ocr_text = &frame.ocr_text;
                            let index_time = frame.index_time;

                            // **E23 修复（第三轮）**：旧实现用 `timestamp.to_bits()` 做 key，
                            // 而 index_cmd 写 text vectors 时做过 `(ts * 1000).round()`，
                            // f32→f64→f32 的精度漂移会让 to_bits 不匹配 ⇒ 重索引后备注和 OCR 没了。
                            // 改用毫秒级整数键，免疫亚毫秒浮点差异。
                            let ts_ms_key = (timestamp as f64 * 1000.0).round() as i64;
                            let (old_ocr, old_note) = legacy
                                .get(&ts_ms_key)
                                .cloned()
                                .unwrap_or_default();
                            let final_ocr = if ocr_text.is_empty() { old_ocr } else { ocr_text.clone() };
                            let final_note = if old_note.is_empty() {
                                legacy_file_note.clone()
                            } else {
                                old_note
                            };
                            tx.execute(
                                "INSERT INTO frame_vectors
                                    (path, timestamp, vector_f32, indexed_in_batch_id, ocr_text, user_note, index_time)
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                                params![
                                    path,
                                    timestamp,
                                    encode_f32_le(vec),
                                    batch_id.clone(),
                                    final_ocr,
                                    final_note,
                                    index_time
                                ],
                            )?;
                            }

                            // ⑤ 行级 OCR 落库：把该文件所有新帧的行级结果一次性覆盖写入
                            //    （③ 已随帧删除旧行，这里只负责补上新的）。
                            let all_lines: Vec<crate::model_code::OcrLine> = entries
                            .iter()
                            .flat_map(|f| f.ocr_lines.iter().cloned())
                            .collect();
                            crate::storage::ocr_store::replace_ocr_entries(&tx, path, &all_lines)
                            .map_err(to_sql_error)?;

                            Ok(())
                    })();

                    match result {
                        Ok(_) => match tx.commit() {
                            Ok(_) => {
                                // 内存矩阵同步：先移除旧帧，再压入新帧。
                                // 备注同样继承，避免内存与库不一致。
                                memory.remove_by_paths(&[path.clone()]);
                                for frame in entries {
                                    let note = read_file_note(&db, path);
                                    memory.push(
                                        path.clone(),
                                        frame.timestamp,
                                        frame.vector,
                                        frame.ocr_text,
                                        note,
                                        frame.index_time,
                                    );
                                }

                                replaced_frames += old_count;
                                new_frames += inserted_count;

                                // 只有真正成功完成，才写 content_modified。
                                let _ = journal::record_event(
                                    &db,
                                    &batch_id,
                                    path,
                                    EventType::ContentModified,
                                    Some(
                                        &json!({
                                            "old_frame_count": old_count,
                                            "new_frame_count": inserted_count
                                        })
                                        .to_string(),
                                    ),
                                );
                                let _ = journal::upsert_file_observed(
                                    &db,
                                    path,
                                    journal::stat_file(path),
                                );
                                succeeded.push(path.clone());
                            }
                            Err(e) => {
                                let _ = journal::record_event(
                                    &db,
                                    &batch_id,
                                    path,
                                    EventType::ReindexFailed,
                                    Some(&json!({ "error": e.to_string() }).to_string()),
                                );
                                failed.push(ReindexFailure {
                                    path: path.clone(),
                                    error: e.to_string(),
                                });
                            }
                        },
                        Err(e) => {
                            let _ = tx.rollback();
                            let _ = journal::record_event(
                                &db,
                                &batch_id,
                                path,
                                EventType::ReindexFailed,
                                Some(&json!({ "error": e.to_string() }).to_string()),
                            );
                            failed.push(ReindexFailure {
                                path: path.clone(),
                                error: e.to_string(),
                            });
                        }
                    }
                }
            }
            Err(e) => {
                // 整批请求失败：该批每个路径都记为失败，等待下次重试
                for path in chunk {
                    let _ = journal::record_event(
                        &db,
                        &batch_id,
                        path,
                        EventType::ReindexFailed,
                        Some(&json!({ "error": e.clone() }).to_string()),
                    );
                    failed.push(ReindexFailure {
                        path: path.clone(),
                        error: e.clone(),
                    });
                }
            }
        }
    }

    let summary = json!({
        "succeeded": succeeded.len(),
        "failed": failed.len(),
        "replaced_frames": replaced_frames,
        "new_frames": new_frames,
    })
    .to_string();
    if let Err(e) = journal::finish_batch(&db, &batch_id, &summary) {
        println!("⚠️ Failed to close reindex batch {}: {}", batch_id, e);
    }

    fs_trace!(
        "reindex done, batch_id={}, succeeded={}, failed={}, replaced_frames={}, new_frames={}",
        batch_id,
        succeeded.len(),
        failed.len(),
        replaced_frames,
        new_frames
    );

    Ok(ReindexResult {
        batch_id,
        succeeded,
        failed,
        replaced_frames,
        new_frames,
    })
}

/// 读取某路径旧帧的 (ocr_text, user_note)，按 `timestamp` 的位模式索引。
///
/// 读出旧帧的备注与 OCR 文本，供重索引时继承。
///
/// **E23 修复（第三轮）**：旧实现用 `f32.to_bits()` 做 key，但 worker 产出的
/// timestamp 与库里存的 f64→f32 转换可能有亚毫秒浮点漂移，导致 to_bits 不匹配、
/// 继承失败（重索引后备注和 OCR 没了）。改用毫秒级整数键 `(ts * 1000).round()`，
/// 与 index_cmd 写 text vectors 时的 round 口径一致，免疫亚毫秒差异。
fn read_legacy_frame_meta(
    conn: &rusqlite::Connection,
    path: &str,
) -> Result<HashMap<i64, (String, String)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT timestamp, ocr_text, user_note FROM frame_vectors WHERE path = ?1",
    )?;
    let rows = stmt.query_map(params![path], |row| {
        Ok((
            row.get::<_, f64>(0)? as f32,
            row.get::<_, String>(1).unwrap_or_default(),
            row.get::<_, String>(2).unwrap_or_default(),
        ))
    })?;
    let mut out = HashMap::new();
    for row in rows.flatten() {
        let (ts, ocr, note) = row;
        let ts_ms = (ts as f64 * 1000.0).round() as i64;
        out.insert(ts_ms, (ocr, note));
    }
    Ok(out)
}

/// 文件级备注兜底：取该路径任意一个非空备注（备注本是文件级的，各帧一致）。
fn read_file_note(conn: &rusqlite::Connection, path: &str) -> String {
    conn.query_row(
        "SELECT user_note FROM frame_vectors WHERE path = ?1 AND user_note <> '' LIMIT 1",
        params![path],
        |row| row.get::<_, String>(0),
    )
    .unwrap_or_default()
}

/// 把 journal 的 String 错误转成 rusqlite::Error，使其纳入重索引事务的统一回滚。
fn to_sql_error(msg: String) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(msg)))
}
