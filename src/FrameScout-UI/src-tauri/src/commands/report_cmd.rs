// 扫描报告 / 活动记录：把 batches 表里的骨架与摘要读出来给用户看。
//
// 数据来源就是迁移链建立的 `batches` 表——不额外建表、不额外记账。
// **合法完成的批次**（`finished_at` 非空）才算数；崩溃残留已在启动时标为
// interrupted，这里照常返回，让用户能看见「那次没跑完」。

use tauri::State;

use crate::fs_trace;
use crate::error::{ec1, ERR_DB_PREPARE_FAILED, ERR_DB_QUERY_FAILED, ERR_LOCK_POISONED};
use crate::model_code::{AppState, BatchReport};

/// 取最近的批次记录（按 id 倒序，最新的在前）。
/// `batch_type` 可选过滤（M1'''' P0：来源批次下拉只应列出「扫描」批次——
/// `clean_ghosts` / `legacy_import` 不写 frame_vectors，选中必然 0 结果）。
///
/// ⚠️ 第八轮：默认（None / 空串）返回**全部类型**，因为「扫描报告」对话框要看到
/// 重索引、幽灵清理等所有批次。两个调用方都必须显式表达意图：
///  - 搜索栏「来源批次」下拉 → 传 `Some("scan")`；
///  - 扫描报告对话框 → 传 `None`（要全部）。
/// 这里刻意不把默认改成 scan：那会让报告对话框悄悄少显示几类批次。
#[tauri::command]
pub async fn list_scan_reports(
    state: State<'_, AppState>,
    limit: Option<u32>,
    batch_type: Option<String>,
) -> Result<Vec<BatchReport>, String> {
    let db = state.db_conn.lock().map_err(|e| ec1(ERR_LOCK_POISONED, e))?;
    let limit = limit.unwrap_or(30).clamp(1, 200);

    // 先多取一批（批次数量少、不会超过 500），Rust 侧按类型过滤后再截断到 limit，
    // 避免动态拼 SQL 带来的 borrow 生命周期问题。
    let fetch_limit: u32 = 500;
    let mut stmt = db
        .prepare(
            "SELECT id, batch_id, batch_type, started_at, finished_at, status, summary_json
             FROM batches ORDER BY id DESC LIMIT ?1",
        )
        .map_err(|e| ec1(ERR_DB_PREPARE_FAILED, e))?;

    let mut items: Vec<BatchReport> = stmt
        .query_map(rusqlite::params![fetch_limit], map_batch_report)
        .map_err(|e| ec1(ERR_DB_QUERY_FAILED, e))?
        .flatten()
        .filter(|b| match batch_type.as_deref() {
            Some(bt) if !bt.is_empty() => b.batch_type == bt,
            _ => true,
        })
        .collect();
    items.truncate(limit as usize);
    drop(stmt);

    fs_trace!("list scan reports, limit={}, type={:?}", limit, batch_type);
    Ok(items)
}

/// 把 `batches` 的一行映射成 `BatchReport`。
fn map_batch_report(row: &rusqlite::Row) -> rusqlite::Result<BatchReport> {
    Ok(BatchReport {
        id: row.get(0)?,
        batch_id: row.get(1)?,
        batch_type: row.get(2)?,
        started_at: row.get(3)?,
        finished_at: row.get(4)?,
        status: row.get(5)?,
        summary_json: row.get(6)?,
    })
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
    let db = state.db_conn.lock().map_err(|e| ec1(ERR_LOCK_POISONED, e))?;

    // 默认返回最近 365 天
    let lookback = days.unwrap_or(365);

    // file_events.occurred_at 存的是 ISO-8601 字符串（utc_now_iso()，形如
    // `YYYY-MM-DDTHH:MM:SSZ`），不是 Unix 秒。直接用 date(occurred_at) 取 UTC
    // 日期前缀做聚合；只有需要 Unix 秒时才用 strftime('%s', ...) 转换。
    let mut stmt = db
        .prepare(
            "SELECT date(occurred_at) AS day,
                    COUNT(*) AS cnt,
                    CAST(strftime('%s', MIN(occurred_at)) AS INTEGER) AS first_ts
             FROM file_events
             WHERE event_type = 'first_seen'
               AND date(occurred_at) >= date('now', '-' || ?1 || ' days')
             GROUP BY day
             ORDER BY day DESC",
        )
        .map_err(|e| ec1(ERR_DB_PREPARE_FAILED, e))?;

    let items: Vec<TimelineDay> = stmt
        .query_map([lookback], |row| {
            Ok(TimelineDay {
                date: row.get(0)?,
                count: row.get(1)?,
                timestamp: row.get(2)?,
            })
        })
        .map_err(|e| ec1(ERR_DB_QUERY_FAILED, e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(items)
}

/// P2-4：「去年今天」——查询一年前同日入库的文件数。
#[tauri::command]
pub async fn last_year_today_count(state: State<'_, AppState>) -> Result<i64, String> {
    let db = state.db_conn.lock().map_err(|e| ec1(ERR_LOCK_POISONED, e))?;

    // 一年前的今天：减 365 天，取当天的 first_seen 事件数。
    // occurred_at 是 ISO 字符串，直接用 date(occurred_at) 取 UTC 日期比较。
    let count: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM file_events
             WHERE event_type = 'first_seen'
               AND date(occurred_at) = date('now', '-365 days')",
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
    let db = state.db_conn.lock().map_err(|e| ec1(ERR_LOCK_POISONED, e))?;

    let mut stmt = db
        .prepare(
            "SELECT path, CAST(strftime('%s', MIN(occurred_at)) AS INTEGER) AS ts
             FROM file_events
             WHERE event_type = 'first_seen'
               AND date(occurred_at) = ?1
             GROUP BY path
             ORDER BY ts DESC",
        )
        .map_err(|e| ec1(ERR_DB_PREPARE_FAILED, e))?;

    let items: Vec<TimelineFile> = stmt
        .query_map([&date], |row| {
            Ok(TimelineFile {
                path: row.get(0)?,
                timestamp: row.get(1)?,
            })
        })
        .map_err(|e| ec1(ERR_DB_QUERY_FAILED, e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(items)
}
