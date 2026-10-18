// 备份 / 恢复 / 完整性校验（P2-5 / 第三轮「备份/恢复三档」）。
//
// 规划里的承诺：「迁移机器等于搬家，不当难民」。
// 本轮先兑现骨架：立即备份（带时间戳 .db 副本，自动轮转 3 份）+
// 完整性校验（PRAGMA integrity_check）。恢复留接口（需关 App 再覆盖文件，
// 放到后续，因为恢复涉及进程锁问题）。

use std::fs;
use std::path::PathBuf;
use tauri::State;
use crate::error::{
    ec, ec1, ERR_BACKUP_DELETE_FAILED, ERR_BACKUP_DELETE_NOT_DB, ERR_BACKUP_DIR_CREATE_FAILED,
    ERR_BACKUP_DIR_UNKNOWN, ERR_BACKUP_FAILED, ERR_BACKUP_FILE_NOT_FOUND, ERR_BACKUP_LIST_FAILED,
    ERR_BACKUP_NOT_DB_FILE, ERR_BACKUP_REFUSE_NON_BACKUP, ERR_DB_FILE_NOT_FOUND,
    ERR_DB_INTEGRITY_FAILED, ERR_DB_OPEN_FAILED, ERR_DB_PATH_UNKNOWN, ERR_DIR_READ_FAILED,
    ERR_LOCK_POISONED, ERR_RESTORE_MARKER_FAILED,
};
use crate::model_code::AppState;
use crate::storage::app_settings;
use crate::fs_trace;

/// 立即备份的返回：新备份路径 + 轮转中被删除的旧备份文件名（M0''' 告知）。
#[derive(serde::Serialize)]
pub struct BackupResult {
    pub path: String,
    pub removed: Vec<String>,
}

/// 恢复前自动备份 + 帧数对比的返回（M1'''' P0-2：告知「当前 N 条 → 备份时 M 条」）。
#[derive(serde::Serialize)]
pub struct RestoreResult {
    /// 恢复前自动备份的当前库路径（回滚点）
    pub auto_backup_path: String,
    /// 当前库的帧数（恢复前）
    pub current_frames: i64,
    /// 目标备份的帧数（恢复后）
    pub backup_frames: i64,
}

/// 恢复预览（只读，不备份、不写标记）：供确认框显示「会丢多少」。
#[derive(serde::Serialize)]
pub struct RestorePreview {
    pub current_frames: i64,
    pub backup_frames: i64,
}

/// 备份核心逻辑（供 `backup_database` 与 `restore_backup` 复用）：
/// 复制当前库为带时间戳的 `.db` 副本，按 `keep` 轮转（`-1` = 不限），返回路径与被删文件。
fn do_backup(db_path: &str, keep: i64, exclude: Option<&str>) -> Result<BackupResult, String> {
    if db_path.is_empty() {
        return Err(ec(ERR_DB_PATH_UNKNOWN));
    }

    let src = PathBuf::from(db_path);
    if !src.exists() {
        return Err(ec1(ERR_DB_FILE_NOT_FOUND, db_path));
    }

    let backup_dir = src
        .parent()
        .map(|p| p.join("backups"))
        .ok_or_else(|| ec(ERR_BACKUP_DIR_UNKNOWN))?;
    fs::create_dir_all(&backup_dir).map_err(|e| ec1(ERR_BACKUP_DIR_CREATE_FAILED, e))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let backup_name = format!("manual_backup_{}.db", now);
    let backup_path = backup_dir.join(&backup_name);

    // 复制（含 WAL 文件）
    fs::copy(&src, &backup_path).map_err(|e| ec1(ERR_BACKUP_FAILED, e))?;
    let wal_path = src.with_extension("db-wal");
    if wal_path.exists() {
        let wal_backup = backup_path.with_extension("db-wal");
        let _ = fs::copy(&wal_path, &wal_backup);
    }

    let removed = rotate_backups(&backup_dir, "manual_backup_", keep, exclude);

    let size = fs::metadata(&backup_path).map(|m| m.len()).unwrap_or(0);
    fs_trace!("manual backup created: {} ({} bytes), removed {}", backup_name, size, removed.len());

    Ok(BackupResult {
        path: backup_path.to_string_lossy().to_string(),
        removed,
    })
}

/// 统计某个 `.db` 文件里的帧数（供恢复前对比「会丢多少」）。
fn count_frames_of(path: &str) -> Result<i64, String> {
    let conn = rusqlite::Connection::open(path)
        .map_err(|e| ec1(ERR_DB_OPEN_FAILED, e))?;
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM frame_vectors", [], |row| row.get(0))
        .unwrap_or(0);
    Ok(count)
}

/// 立即备份：把当前数据库文件复制一份带时间戳的 .db 副本。
/// 自动轮转：只保留最近 N 份（N 由用户设置，`-1` = 不限），超出删除并回传被删文件名。
#[tauri::command]
pub async fn backup_database(state: State<'_, AppState>) -> Result<BackupResult, String> {
    let db_path = state.db_path.lock().map_err(|e| ec1(ERR_LOCK_POISONED, e))?.clone();
    let keep = {
        let db = state.db_conn.lock().map_err(|e| ec1(ERR_LOCK_POISONED, e))?;
        app_settings::get_i64(&db, app_settings::KEY_BACKUP_KEEP_COUNT, 3)
    };
    do_backup(&db_path, keep, None)
}

/// 完整性校验：PRAGMA integrity_check。
#[tauri::command]
pub async fn verify_integrity(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db_conn.lock().map_err(|e| ec1(ERR_LOCK_POISONED, e))?;
    let result: String = db
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| ec1(ERR_DB_INTEGRITY_FAILED, e))?;
    Ok(result)
}

/// 列出已有备份。
#[tauri::command]
pub async fn list_backups(state: State<'_, AppState>) -> Result<Vec<BackupInfo>, String> {
    let db_path = state.db_path.lock().map_err(|e| ec1(ERR_LOCK_POISONED, e))?.clone();
    if db_path.is_empty() {
        return Ok(Vec::new());
    }
    let src = PathBuf::from(&db_path);
    let backup_dir = src.parent()
        .map(|p| p.join("backups"))
        .ok_or_else(|| ec(ERR_BACKUP_DIR_UNKNOWN))?;

    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();
    for entry in fs::read_dir(&backup_dir).map_err(|e| ec1(ERR_BACKUP_LIST_FAILED, e))? {
        let entry = entry.map_err(|e| ec1(ERR_DIR_READ_FAILED, e))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".db") {
            continue;
        }
        let path = entry.path().to_string_lossy().to_string();
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let modified = entry.metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        backups.push(BackupInfo { name, path, size_bytes: size, modified_ts: modified });
    }
    // 按修改时间降序
    backups.sort_by(|a, b| b.modified_ts.cmp(&a.modified_ts));
    Ok(backups)
}

#[derive(serde::Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_ts: i64,
}

/// 恢复指定备份（M2''' 补上「出」，结束「单程票」）。
///
/// 采用「pending restore」方案：恢复需要关闭当前持活连接的数据库文件，
/// 覆盖 `.db` 会损坏，故本命令只校验备份合法性并写入待恢复标记，
/// 提示用户「下次启动生效」。启动时 `init_db_and_load_memory` 打开数据库前
/// 检查标记，若有则先用备份替换主库再打开。
/// 预览（只读）：返回「当前库帧数 vs 备份帧数」，供确认框显示「会丢多少」。
#[tauri::command]
pub async fn preview_restore(state: State<'_, AppState>, path: String) -> Result<RestorePreview, String> {
    let normalized = path.replace('/', "\\");
    if !normalized.ends_with(".db") {
        return Err(ec(ERR_BACKUP_NOT_DB_FILE));
    }
    if !normalized.to_lowercase().contains("backups") {
        return Err(ec(ERR_BACKUP_REFUSE_NON_BACKUP));
    }
    let target = PathBuf::from(&normalized);
    if !target.exists() {
        return Err(ec1(ERR_BACKUP_FILE_NOT_FOUND, normalized));
    }

    let db_path = state.db_path.lock().map_err(|e| ec1(ERR_LOCK_POISONED, e))?.clone();
    let current_frames = count_frames_of(&db_path)?;
    let backup_frames = count_frames_of(&normalized)?;
    Ok(RestorePreview { current_frames, backup_frames })
}

#[tauri::command]
pub async fn restore_backup(state: State<'_, AppState>, path: String) -> Result<RestoreResult, String> {
    let normalized = path.replace('/', "\\");
    if !normalized.ends_with(".db") {
        return Err(ec(ERR_BACKUP_NOT_DB_FILE));
    }
    if !normalized.to_lowercase().contains("backups") {
        return Err(ec(ERR_BACKUP_REFUSE_NON_BACKUP));
    }
    let target = PathBuf::from(&normalized);
    if !target.exists() {
        return Err(ec1(ERR_BACKUP_FILE_NOT_FOUND, normalized));
    }

    // M1'''' P0-2：恢复前自动备份当前库（作为回滚点），避免「一次点击永久丢索引」。
    let db_path = state.db_path.lock().map_err(|e| ec1(ERR_LOCK_POISONED, e))?.clone();
    let keep = {
        let db = state.db_conn.lock().map_err(|e| ec1(ERR_LOCK_POISONED, e))?;
        app_settings::get_i64(&db, app_settings::KEY_BACKUP_KEEP_COUNT, 3)
    };
    // 第八轮核查修复：把「要恢复的目标备份」从轮转里排除，否则自动备份会按 mtime
    // 把它当成最旧备份删掉，下次启动恢复静默失效。
    let auto_backup = do_backup(&db_path, keep, Some(&normalized))?;

    // 帧数对比：当前 N 条 vs 备份 M 条，前端据此告知「将丢失/增加 X 条」。
    let current_frames = count_frames_of(&db_path)?;
    let backup_frames = count_frames_of(&normalized)?;

    // 写入待恢复标记：下次启动时用该备份替换主库。
    let marker = pending_restore_marker_path();
    if let Some(dir) = marker.parent() {
        let _ = fs::create_dir_all(dir);
    }
    fs::write(&marker, &normalized).map_err(|e| ec1(ERR_RESTORE_MARKER_FAILED, e))?;
    fs_trace!(
        "restore marked (pending): {} (current {} frames -> backup {} frames)",
        normalized,
        current_frames,
        backup_frames
    );
    Ok(RestoreResult {
        auto_backup_path: auto_backup.path,
        current_frames,
        backup_frames,
    })
}

/// 待恢复标记文件的路径（与主库同目录的 `pending_restore.txt`）。
pub fn pending_restore_marker_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("FrameScout-Offline_AI_Search-Global")
        .join("pending_restore.txt")
}

/// 删除指定备份文件（第五轮 P2-8：此前「能建不能删」，只能靠自动轮转淘汰）。
/// 安全护栏：只允许删除 `backups/` 目录下的 `.db` 文件，防止误删其他文件。
#[tauri::command]
pub async fn delete_backup(path: String) -> Result<(), String> {
    let target = PathBuf::from(&path);
    let normalized = target.to_string_lossy().to_string();
    if !normalized.ends_with(".db") {
        return Err(ec(ERR_BACKUP_DELETE_NOT_DB));
    }
    if !normalized.contains("backups") {
        return Err(ec(ERR_BACKUP_REFUSE_NON_BACKUP));
    }
    if !target.exists() {
        return Ok(()); // 已不存在，幂等
    }
    fs::remove_file(&target).map_err(|e| ec1(ERR_BACKUP_DELETE_FAILED, e))?;
    // 同步清理可能存在的 WAL 副本
    let wal = target.with_extension("db-wal");
    if wal.exists() {
        let _ = fs::remove_file(&wal);
    }
    fs_trace!("backup deleted: {}", normalized);
    Ok(())
}

/// 轮转备份文件，只保留最近 N 份。`keep < 0` 表示「不限份数」，跳过裁剪。
/// `exclude` = 不参与轮转的备份文件（绝对路径）——恢复前自动备份时，用它保护
/// 「用户正要恢复的那个目标备份」，避免它被当成最旧备份挤掉。
/// 返回被删除的备份文件名（M0''' 告知：让用户知道被删了什么）。
fn rotate_backups(dir: &std::path::Path, prefix: &str, keep: i64, exclude: Option<&str>) -> Vec<String> {
    if keep < 0 {
        return Vec::new();
    }
    let exclude_name = exclude
        .and_then(|p| std::path::Path::new(p).file_name())
        .map(|s| s.to_string_lossy().to_string());
    let mut files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(prefix) && name.ends_with(".db") {
                // 第八轮核查修复：跳过用户正要恢复的目标备份，绝不删除它。
                if let Some(ex) = &exclude_name {
                    if name == *ex {
                        continue;
                    }
                }
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        files.push((entry.path(), modified));
                    }
                }
            }
        }
    }
    // 按修改时间降序
    files.sort_by(|a, b| b.1.cmp(&a.1));
    // 删除超出 keep 数量的旧备份，记录文件名
    let mut removed = Vec::new();
    for (path, _) in files.iter().skip(keep as usize) {
        let name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        if fs::remove_file(path).is_ok() {
            removed.push(name);
        }
    }
    removed
}
