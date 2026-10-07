// 负责对指定选中文件进行针对性的 OCR 识别并更新数据库/内存元数据。

use rusqlite::params;
use tauri::State;
use crate::constants::RPC_TIMEOUT_INDEX_BASE_MS;
use crate::model_code::AppState;
use crate::proto::framescout as proto;
use crate::services::request_vector;

#[tauri::command]
pub async fn run_ocr_for_selected_files(
    state: State<'_, AppState>,
    file_paths: Vec<String>,
    languages: Vec<String>,
) -> Result<usize, String> {
    if file_paths.is_empty() {
        return Ok(0);
    }

    // OCR 配置内嵌进载荷（协议演进方向：配置与载荷绑定，不再靠注释约定配对）。
    // `languages` 字段保留仅为兼容旧 worker，新实现一律以 ocr_config 为准。
    let ocr_task_payload = proto::SingleOcrTask {
        file_paths: file_paths.clone(),
        languages: languages.clone(),
        ocr_config: Some(proto::OcrConfig {
            enable_ocr: Some(true),
            languages: languages.clone(),
        }),
    };
    let payload = proto::encode_request::Payload::OcrTask(ocr_task_payload);

    // 债单 A13：超时 -1（无限等待）换常量，OCR 卡住时快速失败而非永久转圈。
    let frames = request_vector(payload, RPC_TIMEOUT_INDEX_BASE_MS)?;
    let db = state.db_conn.lock().unwrap();
    let mut memory = state.memory_db.write().map_err(|e| e.to_string())?;

    let mut updated_count = 0;
    // OCR 任务只关心识别文本，因此刻意忽略向量。
    //
    // ⚠️ 必须按 (path, timestamp) 复合主键更新：一个视频有 N 帧就有 N 行，
    // 若只按 path 更新，最后一帧的文本会覆盖前面所有帧，整段视频只剩一句 OCR。
    let mut lines_by_path: std::collections::HashMap<String, Vec<crate::model_code::OcrLine>> =
        std::collections::HashMap::new();

    for frame in frames {
        let (file_path, timestamp, extracted_text) =
            (frame.path, frame.timestamp, frame.ocr_text);

        if db.execute(
            "UPDATE frame_vectors SET ocr_text = ?1 WHERE path = ?2 AND timestamp = ?3",
            params![extracted_text.clone(), file_path.clone(), timestamp],
        ).is_ok() {
            for meta in memory.metadata.iter_mut() {
                if meta.path == file_path && meta.timestamp == timestamp {
                    meta.ocr_text = extracted_text.clone();
                }
            }
            updated_count += 1;
        }

        if !frame.ocr_lines.is_empty() {
            lines_by_path.entry(file_path).or_default().extend(frame.ocr_lines);
        }
    }

    // 行级 OCR 覆盖落库（与 ocr_text 更新保持同源）
    for (path, lines) in lines_by_path {
        if let Err(e) = crate::storage::ocr_store::replace_ocr_entries(&db, &path, &lines) {
            println!("⚠️ Failed to store OCR entries for {}: {}", path, e);
        }
    }

    Ok(updated_count)
}