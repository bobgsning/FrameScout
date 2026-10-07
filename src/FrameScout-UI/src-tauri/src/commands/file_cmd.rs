// 负责单项备注修改、幽灵文件清理（失效路径清理）、以及全量/分页文件列表查询。

use rusqlite::params;
use tauri::State;
use crate::model_code::{AppState, PagedResponse, SearchResult};
use crate::storage::{normalize_path, path_key};

/// 更新用户备注。
///
/// P1-13 / P1-15 修复（第三轮）：
///   - 补 `normalize_path`（旧实现是唯一没做路径规范化的写路径，
///     UPDATE 命中 0 行但返回 Ok，「我加了备注没报错却搜不到」）；
///   - 新增可选 `timestamp` 参数：视频帧级备注精确到帧（旧实现整视频共享一条）。
///
/// 债单 B13/B14 修复：
///   - 语义澄清：不传 `timestamp` = **全文件笔记**（覆盖该文件所有帧），
///     传 `timestamp` = 帧级笔记（仅该帧）。两套语义有明确区别，调用方按需选择。
///   - 内存同步改用 `path_key`（大小写折叠）比较，与 P1-15 的去重口径一致，
///     不再用精确 path 比较（同一项目里两套「什么算同一个文件」的判断）。
#[tauri::command]
pub async fn update_note(
    state: State<'_, AppState>,
    path: String,
    note: String,
    timestamp: Option<f32>,
) -> Result<(), String> {
    let normalized = normalize_path(&path);
    let db = state.db_conn.lock().unwrap();

    // 有 timestamp：只更新特定帧的备注（视频帧级精确标注）
    // 无 timestamp：全文件笔记（覆盖该文件所有帧）
    let affected = if let Some(ts) = timestamp {
        db.execute(
            "UPDATE frame_vectors SET user_note = ?1 WHERE path = ?2 AND timestamp = ?3",
            params![note.clone(), normalized.clone(), ts],
        )
        .map_err(|e| e.to_string())?
    } else {
        db.execute(
            "UPDATE frame_vectors SET user_note = ?1 WHERE path = ?2",
            params![note.clone(), normalized.clone()],
        )
        .map_err(|e| e.to_string())?
    };

    // 同步内存矩阵（债单 B14：用 path_key 比较，而非精确 path）
    let mut memory = state.memory_db.write().map_err(|e| e.to_string())?;
    let key = path_key(&normalized);
    for meta in memory.metadata.iter_mut() {
        if path_key(&meta.path) == key {
            if let Some(ts) = timestamp {
                // 只更新匹配 timestamp 的帧
                if (meta.timestamp - ts).abs() < 0.01 {
                    meta.user_note = note.clone();
                }
            } else {
                meta.user_note = note.clone();
            }
        }
    }

    if affected == 0 {
        // P1-15：旧实现 UPDATE 命中 0 行也返回 Ok，用户以为成功了
        return Err(format!("No matching record found (path={}, ts={:?}); note not saved", normalized, timestamp));
    }
    Ok(())
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
            "SELECT path, timestamp, ocr_text, user_note, index_time
             FROM frame_vectors
             ORDER BY index_time DESC
             LIMIT ?1 OFFSET ?2",
        )
        .map_err(|e| format!("Prepare error: {}", e))?;

    let items = stmt
        .query_map(params![limit, offset], |row| {
            let path: String = row.get(0)?;
            let timestamp: f64 = row.get(1)?;
            let ocr_text: String = row.get(2)?;
            let user_note: String = row.get(3).unwrap_or_default();
            let index_time: f64 = row.get(4).unwrap_or(0.0);
            Ok(SearchResult {
                path,
                timestamp: timestamp as f32,
                score: 2.0,
                matched_tags: vec!["📂 All Files".to_string()],
                ocr_text,
                user_note,
                index_time,
                ocr_lines: Vec::new(),
                match_count: 0,
                // 浏览模式：无视觉、无字面命中。
                visual_similarity: 0.0,
                ocr_hit: false,
                note_hit: false,
                filename_hit: false,
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
            "SELECT path, timestamp, ocr_text, user_note, index_time
             FROM frame_vectors
             ORDER BY index_time DESC",
        )
        .map_err(|e| format!("Prepare error: {}", e))?;

    let items = stmt
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let timestamp: f64 = row.get(1)?;
            let ocr_text: String = row.get(2)?;
            let user_note: String = row.get(3).unwrap_or_default();
            let index_time: f64 = row.get(4).unwrap_or(0.0);
            Ok(SearchResult {
                path,
                timestamp: timestamp as f32,
                score: 2.0,
                matched_tags: vec!["📂 All Files".to_string()],
                ocr_text,
                user_note,
                index_time,
                ocr_lines: Vec::new(),
                match_count: 0,
                // 浏览模式：无视觉、无字面命中。
                visual_similarity: 0.0,
                ocr_hit: false,
                note_hit: false,
                filename_hit: false,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(items)
}