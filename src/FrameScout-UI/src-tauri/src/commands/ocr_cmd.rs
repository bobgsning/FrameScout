// 负责对指定选中文件进行针对性的 OCR 识别并更新数据库/内存元数据。

use rusqlite::params;
use tauri::State;
use crate::models::AppState;
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

    let ocr_task_payload = proto::SingleOcrTask {
        file_paths: file_paths.clone(),
        languages: languages.clone(),
    };
    let payload = proto::encode_request::Payload::OcrTask(ocr_task_payload);

    let frames = request_vector(payload, -1)?;
    let db = state.db_conn.lock().unwrap();
    let mut memory = state.memory_db.write().map_err(|e| e.to_string())?;

    let mut updated_count = 0;
    // 返回元组顺序: (path, timestamp, vector, ocr_text, index_time)。
    // OCR 任务只关心识别文本，因此刻意忽略第 2/3/5 个字段（向量与时间戳）。
    for (file_path, _, _, extracted_text, _) in frames {
        if db.execute(
            "UPDATE frame_vectors SET ocr_text = ?1 WHERE path = ?2",
            params![extracted_text.clone(), file_path.clone()],
        ).is_ok() {
            for meta in memory.metadata.iter_mut() {
                if meta.path == file_path {
                    meta.ocr_text = extracted_text.clone();
                }
            }
            updated_count += 1;
        }
    }

    Ok(updated_count)
}