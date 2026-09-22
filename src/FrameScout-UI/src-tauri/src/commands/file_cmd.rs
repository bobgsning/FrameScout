// 负责单项备注修改、幽灵文件清理（失效路径清理）、以及全量/分页文件列表查询。

use rusqlite::params;
use tauri::State;
use crate::models::{AppState, PagedResponse, SearchResult};

#[tauri::command]
pub async fn update_note(
    state: State<'_, AppState>,
    path: String,
    note: String,
) -> Result<(), String> {
    let db = state.db_conn.lock().unwrap();
    db.execute(
        "UPDATE frame_vectors SET user_note = ?1 WHERE path = ?2",
        params![note.clone(), path.clone()],
    )
    .map_err(|e| e.to_string())?;

    let mut memory = state.memory_db.write().map_err(|e| e.to_string())?;
    for meta in memory.metadata.iter_mut() {
        if meta.path == path {
            meta.user_note = note.clone();
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn clean_ghosts(state: State<'_, AppState>) -> Result<usize, String> {
    let db = state.db_conn.lock().unwrap();
    let mut memory = state.memory_db.write().map_err(|e| e.to_string())?;
    let mut to_remove = Vec::new();

    for meta in &memory.metadata {
        if !std::path::Path::new(&meta.path).exists() {
            to_remove.push(meta.path.clone());
        }
    }

    for p in &to_remove {
        db.execute("DELETE FROM frame_vectors WHERE path = ?1", params![p])
            .map_err(|e| format!("Failed to delete ghost record: {}", e))?;
    }
    memory.remove_by_paths(&to_remove);
    Ok(to_remove.len())
}

#[tauri::command]
pub fn list_all_files(
    state: State<'_, AppState>,
    page: u32,
    limit: u32,
) -> Result<PagedResponse, String> {
    let db = state.db_conn.lock().map_err(|e| e.to_string())?;
    let offset = (page.saturating_sub(1)) * limit;

    let total_count: i64 = db
        .query_row("SELECT COUNT(*) FROM frame_vectors", [], |row| row.get(0))
        .map_err(|e| format!("DB count error: {}", e))?;
    let total_count = total_count as usize;

    let mut stmt = db
        .prepare(
            "SELECT path, timestamp, vector_json, ocr_text, user_note, index_time
             FROM frame_vectors
             ORDER BY index_time DESC
             LIMIT ?1 OFFSET ?2",
        )
        .map_err(|e| format!("Prepare error: {}", e))?;

    let items = stmt
        .query_map(params![limit, offset], |row| {
            let path: String = row.get(0)?;
            let timestamp: f64 = row.get(1)?;
            let _vector_json: String = row.get(2)?;
            let ocr_text: String = row.get(3)?;
            let user_note: String = row.get(4).unwrap_or_default();
            let index_time: f64 = row.get(5).unwrap_or(0.0);
            Ok(SearchResult {
                path,
                timestamp: timestamp as f32,
                score: 2.0,
                matched_tags: vec!["📂 All Files".to_string()],
                ocr_text,
                user_note,
                index_time,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(PagedResponse { items, total_count })
}

#[tauri::command]
pub fn get_all_files(state: State<'_, AppState>) -> Result<Vec<SearchResult>, String> {
    let db = state.db_conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT path, timestamp, vector_json, ocr_text, user_note, index_time
             FROM frame_vectors
             ORDER BY index_time DESC",
        )
        .map_err(|e| format!("Prepare error: {}", e))?;

    let items = stmt
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let timestamp: f64 = row.get(1)?;
            let _vector_json: String = row.get(2)?;
            let ocr_text: String = row.get(3)?;
            let user_note: String = row.get(4).unwrap_or_default();
            let index_time: f64 = row.get(5).unwrap_or(0.0);
            Ok(SearchResult {
                path,
                timestamp: timestamp as f32,
                score: 2.0,
                matched_tags: vec!["📂 All Files".to_string()],
                ocr_text,
                user_note,
                index_time,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(items)
}