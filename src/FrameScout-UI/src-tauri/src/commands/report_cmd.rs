// 扫描报告 / 活动记录：把 batches 表里的骨架与摘要读出来给用户看。
//
// 数据来源就是迁移链建立的 `batches` 表——不额外建表、不额外记账。
// **合法完成的批次**（`finished_at` 非空）才算数；崩溃残留已在启动时标为
// interrupted，这里照常返回，让用户能看见「那次没跑完」。

use tauri::State;

use crate::fs_trace;
use crate::model_code::{AppState, BatchReport};

/// 取最近的批次记录（按 id 倒序，最新的在前）。
#[tauri::command]
pub async fn list_scan_reports(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<BatchReport>, String> {
    let db = state.db_conn.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(30).clamp(1, 200);

    let mut stmt = db
        .prepare(
            "SELECT id, batch_id, batch_type, started_at, finished_at, status, summary_json
             FROM batches
             ORDER BY id DESC
             LIMIT ?1",
        )
        .map_err(|e| format!("Prepare error: {}", e))?;

    let items = stmt
        .query_map([limit], |row| {
            Ok(BatchReport {
                id: row.get(0)?,
                batch_id: row.get(1)?,
                batch_type: row.get(2)?,
                started_at: row.get(3)?,
                finished_at: row.get(4)?,
                status: row.get(5)?,
                summary_json: row.get(6)?,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?
        .flatten()
        .collect();

    fs_trace!("list scan reports, limit={}", limit);
    Ok(items)
}

/// P2-4：时间轴数据——按日聚合 file_events 的 first_seen 事件。
/// 返回每天的入库文件数，供前端画时间轴 + 「去年今天」提示。
#[derive(serde::Serialize)]
pub struct TimelineDay {
    pub date: String,      // YYYY-MM-DD
    pub count: i64,
    pub timestamp: i64,    // 该日 0 点的 Unix 秒
}

#[tauri::command]
pub async fn list_timeline(
    state: State<'_, AppState>,
    days: Option<i64>,
) -> Result<Vec<TimelineDay>, String> {
    let db = state.db_conn.lock().map_err(|e| e.to_string())?;

    // 默认返回最近 365 天
    let lookback = days.unwrap_or(365);
    let cutoff = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64)
        - lookback * 24 * 3600;

    // file_events 表的 occurred_at 是 Unix 秒（整数）。
    // 按 date(occurred_at) 聚合 first_seen 事件
    let mut stmt = db
        .prepare(
            "SELECT date(occurred_at, 'unixepoch', 'localtime') AS day,
                    COUNT(*) AS cnt,
                    MIN(occurred_at) AS first_ts
             FROM file_events
             WHERE event_type = 'first_seen' AND occurred_at >= ?1
             GROUP BY day
             ORDER BY day DESC",
        )
        .map_err(|e| format!("Prepare error: {}", e))?;

    let items: Vec<TimelineDay> = stmt
        .query_map([cutoff], |row| {
            Ok(TimelineDay {
                date: row.get(0)?,
                count: row.get(1)?,
                timestamp: row.get(2)?,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(items)
}

/// P2-4：「去年今天」——查询一年前同日入库的文件数。
#[tauri::command]
pub async fn last_year_today_count(state: State<'_, AppState>) -> Result<i64, String> {
    let db = state.db_conn.lock().map_err(|e| e.to_string())?;

    // 一年前的今天：减 365 天，取当天的 first_seen 事件数
    let count: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM file_events
             WHERE event_type = 'first_seen'
               AND date(occurred_at, 'unixepoch', 'localtime') = date('now', '-365 days', 'localtime')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    Ok(count)
}

/// 某天入库的文件（P2-9：时间轴下钻——点某天看那天入库了哪些文件）。
#[derive(serde::Serialize)]
pub struct TimelineFile {
    pub path: String,
    pub timestamp: i64,
}

#[tauri::command]
pub async fn list_files_by_day(
    state: State<'_, AppState>,
    date: String,
) -> Result<Vec<TimelineFile>, String> {
    let db = state.db_conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .prepare(
            "SELECT path, MIN(occurred_at) AS ts
             FROM file_events
             WHERE event_type = 'first_seen'
               AND date(occurred_at, 'unixepoch', 'localtime') = ?1
             GROUP BY path
             ORDER BY ts DESC",
        )
        .map_err(|e| format!("Prepare error: {}", e))?;

    let items: Vec<TimelineFile> = stmt
        .query_map([&date], |row| {
            Ok(TimelineFile {
                path: row.get(0)?,
                timestamp: row.get(1)?,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(items)
}
