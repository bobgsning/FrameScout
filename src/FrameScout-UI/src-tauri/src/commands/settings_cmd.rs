// 应用设置命令（M0''' 哲学合规止血：自动清理给开关）。
//
// 生命周期清理策略落在 SQLite `app_settings` 键值表，因为裁剪发生在启动期、
// 前端尚未就绪。这里暴露给前端读写的命令：
//   - `get_cleanup_settings` / `set_cleanup_settings`：三档清理策略
//   - `get_startup_cleanup_report`：启动期清理汇总（供前端 Toast 告知）

use tauri::State;
use crate::model_code::{AppState, StartupCleanupReport};
use crate::storage::app_settings;

/// 三档自动清理策略（与前端设置页一一对应）。
/// `-1` = 「永不 / 不限」。
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct CleanupSettings {
    /// 事件历史保留天数（默认 90，-1 = 永不清理）
    pub event_retention_days: i64,
    /// 备份保留份数（默认 3，-1 = 不限）
    pub backup_keep_count: i64,
    /// 备份保留天数（默认 30，-1 = 不限）
    pub backup_max_age_days: i64,
}

impl Default for CleanupSettings {
    fn default() -> Self {
        Self {
            event_retention_days: 90,
            backup_keep_count: 3,
            backup_max_age_days: 30,
        }
    }
}

/// 读取三档清理策略。缺失项回退默认值。
#[tauri::command]
pub async fn get_cleanup_settings(state: State<'_, AppState>) -> Result<CleanupSettings, String> {
    let db = state.db_conn.lock().map_err(|e| e.to_string())?;
    let d = CleanupSettings::default();
    Ok(CleanupSettings {
        event_retention_days: app_settings::get_i64(
            &db,
            app_settings::KEY_EVENT_RETENTION_DAYS,
            d.event_retention_days,
        ),
        backup_keep_count: app_settings::get_i64(
            &db,
            app_settings::KEY_BACKUP_KEEP_COUNT,
            d.backup_keep_count,
        ),
        backup_max_age_days: app_settings::get_i64(
            &db,
            app_settings::KEY_BACKUP_MAX_AGE_DAYS,
            d.backup_max_age_days,
        ),
    })
}

/// 写入三档清理策略。`-1` = 「永不 / 不限」。
#[tauri::command]
pub async fn set_cleanup_settings(
    state: State<'_, AppState>,
    settings: CleanupSettings,
) -> Result<(), String> {
    let db = state.db_conn.lock().map_err(|e| e.to_string())?;
    app_settings::set_setting(
        &db,
        app_settings::KEY_EVENT_RETENTION_DAYS,
        &settings.event_retention_days.to_string(),
    )?;
    app_settings::set_setting(
        &db,
        app_settings::KEY_BACKUP_KEEP_COUNT,
        &settings.backup_keep_count.to_string(),
    )?;
    app_settings::set_setting(
        &db,
        app_settings::KEY_BACKUP_MAX_AGE_DAYS,
        &settings.backup_max_age_days.to_string(),
    )?;
    Ok(())
}

/// 返回启动期自动清理的汇总报告（前端挂载后调用一次，计数 > 0 时 Toast 告知）。
#[tauri::command]
pub async fn get_startup_cleanup_report(
    state: State<'_, AppState>,
) -> Result<StartupCleanupReport, String> {
    let report = state.startup_cleanup.lock().map_err(|e| e.to_string())?.clone();
    Ok(report)
}
