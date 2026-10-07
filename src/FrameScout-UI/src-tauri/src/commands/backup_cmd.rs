// 备份 / 恢复 / 完整性校验（P2-5 / 第三轮「备份/恢复三档」）。
//
// 规划里的承诺：「迁移机器等于搬家，不当难民」。
// 本轮先兑现骨架：立即备份（带时间戳 .db 副本，自动轮转 3 份）+
// 完整性校验（PRAGMA integrity_check）。恢复留接口（需关 App 再覆盖文件，
// 放到后续，因为恢复涉及进程锁问题）。

use std::fs;
use std::path::PathBuf;
use tauri::State;
use crate::model_code::AppState;
use crate::fs_trace;

/// 立即备份：把当前数据库文件复制一份带时间戳的 .db 副本。
/// 自动轮转：只保留最近 3 份，超出删除。
#[tauri::command]
pub async fn backup_database(state: State<'_, AppState>) -> Result<String, String> {
    let db_path = state.db_path.lock().map_err(|e| e.to_string())?.clone();
    if db_path.is_empty() {
        return Err("Database path unknown".to_string());
    }

    let src = PathBuf::from(&db_path);
    if !src.exists() {
        return Err(format!("Database file not found: {}", db_path));
    }

    // 备份目录：与数据库同级的 backups/
    let backup_dir = src.parent()
        .map(|p| p.join("backups"))
        .ok_or("Cannot determine backup directory")?;
    fs::create_dir_all(&backup_dir).map_err(|e| format!("Failed to create backup directory: {}", e))?;

    // 带时间戳的文件名（用 std::time，避免引入 chrono 依赖）
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let backup_name = format!("manual_backup_{}.db", now);
    let backup_path = backup_dir.join(&backup_name);

    // 复制（含 WAL 文件）
    fs::copy(&src, &backup_path).map_err(|e| format!("Backup failed: {}", e))?;
    let wal_path = src.with_extension("db-wal");
    if wal_path.exists() {
        let wal_backup = backup_path.with_extension("db-wal");
        let _ = fs::copy(&wal_path, &wal_backup);
    }

    // 轮转：保留最近 3 份 manual_backup_*
    rotate_backups(&backup_dir, "manual_backup_", 3);

    let size = fs::metadata(&backup_path)
        .map(|m| m.len())
        .unwrap_or(0);
    fs_trace!("manual backup created: {} ({} bytes)", backup_name, size);

    Ok(backup_path.to_string_lossy().to_string())
}

/// 完整性校验：PRAGMA integrity_check。
#[tauri::command]
pub async fn verify_integrity(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db_conn.lock().map_err(|e| e.to_string())?;
    let result: String = db
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| format!("integrity check failed: {}", e))?;
    Ok(result)
}

/// 列出已有备份。
#[tauri::command]
pub async fn list_backups(state: State<'_, AppState>) -> Result<Vec<BackupInfo>, String> {
    let db_path = state.db_path.lock().map_err(|e| e.to_string())?.clone();
    if db_path.is_empty() {
        return Ok(Vec::new());
    }
    let src = PathBuf::from(&db_path);
    let backup_dir = src.parent()
        .map(|p| p.join("backups"))
        .ok_or("Cannot determine backup directory")?;

    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();
    for entry in fs::read_dir(&backup_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
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

/// 删除指定备份文件（第五轮 P2-8：此前「能建不能删」，只能靠自动轮转淘汰）。
/// 安全护栏：只允许删除 `backups/` 目录下的 `.db` 文件，防止误删其他文件。
#[tauri::command]
pub async fn delete_backup(path: String) -> Result<(), String> {
    let target = PathBuf::from(&path);
    let normalized = target.to_string_lossy().to_string();
    if !normalized.ends_with(".db") {
        return Err("Only .db backup files can be deleted".to_string());
    }
    if !normalized.contains("backups") {
        return Err("Refusing to delete a non-backup file".to_string());
    }
    if !target.exists() {
        return Ok(()); // 已不存在，幂等
    }
    fs::remove_file(&target).map_err(|e| format!("Failed to delete backup: {}", e))?;
    // 同步清理可能存在的 WAL 副本
    let wal = target.with_extension("db-wal");
    if wal.exists() {
        let _ = fs::remove_file(&wal);
    }
    fs_trace!("backup deleted: {}", normalized);
    Ok(())
}

/// 轮转备份文件，只保留最近 N 份。
fn rotate_backups(dir: &std::path::Path, prefix: &str, keep: usize) {
    let mut files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(prefix) && name.ends_with(".db") {
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
    // 删除超出 keep 数量的旧备份
    for (path, _) in files.iter().skip(keep) {
        let _ = fs::remove_file(path);
    }
}
