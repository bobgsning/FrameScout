// =========================================================================
//  应用设置键值表（M0''' 哲学合规止血）
// =========================================================================
//  生命周期清理策略（事件保留天数、备份份数/天数）落在 SQLite 的 `app_settings`
//  键值表——因为裁剪发生在启动期、前端尚未就绪，localStorage 无法在裁剪前读到。
//
//  约定：
//   - 值一律存字符串，由读取方按语义解析；
//   - 「永不 / 不限」用哨兵 `-1` 表示；
//   - 默认值在缺失时兜底（与旧行为一致：事件 90 天、备份 3 份/30 天）。

use rusqlite::{params, Connection};

/// 设置键名。
pub const KEY_EVENT_RETENTION_DAYS: &str = "event_retention_days";
pub const KEY_BACKUP_KEEP_COUNT: &str = "backup_keep_count";
pub const KEY_BACKUP_MAX_AGE_DAYS: &str = "backup_max_age_days";

/// 读取某个设置的原始字符串值。缺失返回 None。
pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        params![key],
        |row| row.get::<_, String>(0),
    )
    .ok()
}

/// 写入（upsert）某个设置。
pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(|e| format!("set_app_setting failed: {}", e))?;
    Ok(())
}

/// 读取 i64 型设置；缺失或解析失败时回退到 `default`。
/// `-1` 是「永不 / 不限」哨兵，调用方需自行判断。
pub fn get_i64(conn: &Connection, key: &str, default: i64) -> i64 {
    match get_setting(conn, key) {
        Some(v) => v.trim().parse::<i64>().unwrap_or(default),
        None => default,
    }
}
