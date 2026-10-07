// 负责智能文件夹的保存、列表统计、直接执行、重命名及删除。

use rusqlite::params;
use tauri::State;
use crate::model_code::{AppState, PagedResponse, SmartFolder};
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

/// 更新智能文件夹（重命名 + 查询条件 + 目标开关）。
/// 不改 id，不改 match_count（match_count 由 get_smart_folders 动态计算）。
#[tauri::command]
pub async fn update_smart_folder(
    state: State<'_, AppState>,
    id: i64,
    name: String,
    query_text: String,
    use_vector: bool,
    use_ocr: bool,
    use_note: bool,
    use_filename: bool,
) -> Result<(), String> {
    let db = state.db_conn.lock().unwrap();
    db.execute(
        "UPDATE smart_folders SET name = ?1, query_text = ?2, use_vector = ?3, use_ocr = ?4, use_note = ?5, use_filename = ?6 WHERE id = ?7",
        params![name, query_text, use_vector as i32, use_ocr as i32, use_note as i32, use_filename as i32, id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_smart_folders(state: State<'_, AppState>) -> Result<Vec<SmartFolder>, String> {
    let db = state.db_conn.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT id, name, query_text, use_vector, use_ocr, use_note, use_filename FROM smart_folders ORDER BY id ASC")
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

        // **P1-3 / E1 修复**：旧实现循环体无 use_vector 分支，纯语义文件夹恒为 0。
        // 字面量通道（ocr/note/filename）走快速内存扫描；use_vector 通道无法在列表
        // 视图里实时跑（需要 ZMQ 编码 query + 全量向量搜索），前端用 `?` 表示
        // 「需执行才知道」，点进去走 execute_smart_folder 拿真实结果。
        // 这里加 use_vector 判定：若文件夹是纯语义（无字面量通道），count 留 0 让
        // 前端显示 `?`；若有混合通道，字面量计数作为下界（用户至少知道有多少条命中）。
        let has_text_channel = use_ocr || use_note || use_filename;

        if !lower_query.is_empty() && has_text_channel {
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

/// 按需刷新单个智能文件夹的真实计数（P1-3）。
/// 前端 hover 或点击 pill 时调用，走完整 search_images 拿 total_count。
/// 对纯语义文件夹（use_vector-only）尤其重要——列表视图只能给 `?`，
/// 这里能给真实数字。代价是跑一次 ZMQ + 向量搜索，故按需而非列表批量调。
#[tauri::command]
pub async fn refresh_smart_folder_count(
    state: State<'_, AppState>,
    id: i64,
) -> Result<usize, String> {
    let (query_text, use_vector, use_ocr, use_note, use_filename) = {
        let db = state.db_conn.lock().map_err(|e| e.to_string())?;
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

    // 走完整检索拿真实 total_count
    let result = search_images(
        state,
        query_text,
        1,
        1, // 只要 total_count，limit=1 最省
        use_vector,
        use_ocr,
        use_note,
        use_filename,
    ).await?;

    Ok(result.total_count)
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