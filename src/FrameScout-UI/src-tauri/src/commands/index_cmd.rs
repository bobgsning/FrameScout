// 负责文件与文件夹扫描、批量调用 AI 向量化并写入数据库与内存矩阵。

use rusqlite::params;
use tauri::{AppHandle, Emitter, State};
use walkdir::WalkDir;

use crate::constants::{IMAGE_EXTENSIONS, VIDEO_EXTENSIONS};
use crate::models::{AppState, ProgressPayload};
use crate::proto::framescout as proto;
use crate::services::request_vector;

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

    let mut added_count = 0;
    let mut current_idx = 0;
    let batch_size = 4;

    for chunk in pending_files.chunks(batch_size) {
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

        let batch_payload = proto::BatchPaths {
            file_paths: chunk.to_vec(),
            ocr_config: Some(proto::OcrConfig {
                enable_ocr,
                languages: ocr_languages.clone(),
            }),
        };
        let payload = proto::encode_request::Payload::Batch(batch_payload);

        match request_vector(payload, -1) {
            Ok(frames) => {
                consecutive_failures = 0;

                let mut db = state.db_conn.lock().unwrap();
                let mut memory = state.memory_db.write().map_err(|e| e.to_string())?;

                let tx_result = db.transaction();
                let mut batch_added_files = Vec::new();

                if let Ok(tx) = tx_result {
                    for (file_path, timestamp, vec, ocr_text, index_time) in frames {
                        let vec_json = serde_json::to_string(&vec).unwrap_or_else(|_| "[]".to_string());
                        if tx.execute(
                            "INSERT INTO frame_vectors (path, timestamp, vector_json, ocr_text, user_note, index_time)
                             VALUES (?1, ?2, ?3, ?4, '', ?5)
                             ON CONFLICT(path, timestamp) DO UPDATE SET
                             vector_json = excluded.vector_json,
                             ocr_text = excluded.ocr_text,
                             index_time = excluded.index_time",
                            params![file_path.clone(), timestamp, vec_json, ocr_text.clone(), index_time],
                        ).is_ok() {
                            memory.push(file_path.clone(), timestamp, vec, ocr_text, "".to_string(), index_time);
                            batch_added_files.push(file_path);
                            added_count += 1;
                        }
                    }
                    let _ = tx.commit();
                }

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

    let _ = app.emit(
        "scan-progress",
        ProgressPayload {
            status: "✅ Done".to_string(),
            file_path: "Queue Finished".to_string(),
            current: total_to_process,
            total: total_to_process,
            new_files: vec![],
        },
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
                let path_str = path.to_string_lossy().to_string();
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
        for path_str in &file_paths {
            if !memory.contains_path(&path_str) {
                pending_files.push(path_str.clone());
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