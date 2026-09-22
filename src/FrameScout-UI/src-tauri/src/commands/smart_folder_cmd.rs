// 负责智能文件夹的保存、列表统计、直接执行及删除。

use rusqlite::params;
use tauri::State;
use crate::models::{AppState, PagedResponse, SmartFolder};
use super::search_cmd::search_images;

#[tauri::command]
pub async fn save_smart_folder(
    state: State<'_, AppState>,
    name: String,
    query_text: String,
    use_vector: bool,
    use_ocr: bool,
    use_note: bool,
    use_filename: bool,
) -> Result<i64, String> {
    let db = state.db_conn.lock().unwrap();
    db.execute(
        "INSERT INTO smart_folders (name, query_text, use_vector, use_ocr, use_note, use_filename) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![name, query_text, use_vector as i32, use_ocr as i32, use_note as i32, use_filename as i32],
    ).map_err(|e| e.to_string())?;
    Ok(db.last_insert_rowid())
}

#[tauri::command]
pub async fn get_smart_folders(state: State<'_, AppState>) -> Result<Vec<SmartFolder>, String> {
    let db = state.db_conn.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT id, name, query_text, use_vector, use_ocr, use_note, use_filename FROM smart_folders")
        .map_err(|e| e.to_string())?;

    let memory = state.memory_db.read().map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let query_text: String = row.get(2)?;
            let v: i32 = row.get(3)?;
            let o: i32 = row.get(4)?;
            let n: i32 = row.get(5)?;
            let f: i32 = row.get(6)?;
            Ok((id, name, query_text, v == 1, o == 1, n == 1, f == 1))
        })
        .map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    let lower_search_list: Vec<(i64, String, String, bool, bool, bool, bool)> = rows.filter_map(|r| r.ok()).collect();

    for (id, name, query_text, use_vector, use_ocr, use_note, use_filename) in lower_search_list {
        let mut count = 0;
        let lower_query = query_text.to_lowercase();

        if !lower_query.is_empty() {
            for meta in &memory.metadata {
                let mut matched = false;
                if use_ocr && !meta.ocr_text.is_empty() && meta.ocr_text.to_lowercase().contains(&lower_query) {
                    matched = true;
                }
                if use_note && !meta.user_note.is_empty() && meta.user_note.to_lowercase().contains(&lower_query) {
                    matched = true;
                }
                if use_filename && meta.path.to_lowercase().contains(&lower_query) {
                    matched = true;
                }
                if matched {
                    count += 1;
                }
            }
        }

        list.push(SmartFolder {
            id,
            name,
            query_text,
            use_vector,
            use_ocr,
            use_note,
            use_filename,
            match_count: count,
        });
    }

    Ok(list)
}

#[tauri::command]
pub async fn execute_smart_folder(
    state: State<'_, AppState>,
    id: i64,
    page: usize,
    limit: usize,
) -> Result<PagedResponse, String> {
    let (query_text, use_vector, use_ocr, use_note, use_filename) = {
        let db = state.db_conn.lock().unwrap();
        db.query_row(
            "SELECT query_text, use_vector, use_ocr, use_note, use_filename FROM smart_folders WHERE id = ?1",
            params![id],
            |row| {
                let q: String = row.get(0)?;
                let v: i32 = row.get(1)?;
                let o: i32 = row.get(2)?;
                let n: i32 = row.get(3)?;
                let f: i32 = row.get(4)?;
                Ok((q, v == 1, o == 1, n == 1, f == 1))
            },
        ).map_err(|e| format!("Smart folder not found: {}", e))?
    };

    search_images(
        state,
        query_text,
        page,
        limit,
        use_vector,
        use_ocr,
        use_note,
        use_filename,
    ).await
}

#[tauri::command]
pub async fn delete_smart_folder(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db_conn.lock().unwrap();
    db.execute("DELETE FROM smart_folders WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}