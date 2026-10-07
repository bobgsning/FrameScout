// 幽灵文件处理：先预览、后行动，且每个动作都有对应的反向动作。
//
// 交互契约（写死，UI 必须遵守）：
//   1. **预览不产生任何副作用**——不写状态、不写事件、不写批次。它只是把事实摆出来。
//   2. **行动由用户触发**：用户勾选路径并明确选择动作，才落库。
//   3. **标记失效可逆**：`mark_dead` 只置 `files.is_dead = 1`，向量行原样保留，
//      因此 `restore` 能让它原封不动回到检索中，无需重新编码。
//   4. **真删除不可逆**：`purge` 删除 `frame_vectors` 与 `files` 的行，
//      是终态动作，因此必须先经过预览。
// 全程读写 `file_events`，每个动作都属于同一个 `clean_ghosts` 批次，
// 便于「按批次查看这次清理动了什么」。

use std::path::Path;

use rusqlite::params;
use serde_json::json;
use tauri::State;

use crate::fs_trace;
use crate::model_code::{AppState, GhostActionResult, GhostItem};
use crate::storage::db::load_frame_rows;
use crate::storage::journal::{self, BatchType, EventType};
use crate::time_util::utc_now_iso;

/// 动作类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GhostAction {
    /// 标记失效：保留向量，退出检索，可回库
    MarkDead,
    /// 回库：恢复参与检索（dead → alive）
    Restore,
    /// 真删除：连同向量行一并移除，不可回退
    Purge,
}

impl GhostAction {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "mark_dead" => Some(GhostAction::MarkDead),
            "restore" => Some(GhostAction::Restore),
            "purge" => Some(GhostAction::Purge),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            GhostAction::MarkDead => "mark_dead",
            GhostAction::Restore => "restore",
            GhostAction::Purge => "purge",
        }
    }
}

/// 抽取路径的「盘根」（P0-1 / 第三轮 D7）。
///
/// - 盘符路径 `C:\Users\bob\a.jpg` → `C:\`
/// - UNC 路径 `\\server\share\dir\file.jpg` → `\\server\share`
/// - 其他形态：原样返回（让 `Path::exists()` 兜底）
///
/// 用来按盘根聚合做一次可达性检测：NAS 未挂载 / 移动硬盘未插时，
/// `Path::exists()` 对该盘上的每个文件都返回 false，但盘根检测一次就能识别
/// 「整个盘离线」而非「每个文件都消失了」——避免误判整盘为幽灵。
fn extract_root(path: &str) -> String {
    let bytes = path.as_bytes();
    // UNC: \\server\share\...
    if path.starts_with("\\\\") {
        // 找第 4 个反斜杠的位置（\\server\share 之后），截取到那里
        let mut bs_count = 0usize;
        let mut end = path.len();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\\' {
                bs_count += 1;
                if bs_count == 4 {
                    end = i;
                    break;
                }
            }
        }
        return path[..end].to_string();
    }
    // 盘符：C:\...
    if bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'\\' {
        return path[..3].to_string();
    }
    path.to_string()
}

/// 预览：库中已失效标记或磁盘上已不存在的文件。
///
/// 返回两类都包含：磁盘不存在但仍在检索中的（`is_dead = 0`），
/// 以及已被标记失效、但仍占着库的（`is_dead = 1`）。
/// 前端据此让用户分别决定「真删除 / 标记失效」与「回库 / 真删除」。
///
/// **NAS 守卫（P0-1 / 第三轮 D7-D8）**：
/// 旧实现裸调 `Path::exists()`，NAS 未挂载时整盘文件被判成幽灵，UI 引导 purge。
/// 新实现按盘根聚合做一次可达性检测，离线盘上的文件标 `disk_offline = true`，
/// 前端整组灰显并禁用 purge——避免误删只是暂时离线的整个盘。
///
/// **last_seen_at 过滤（兑现 journal.rs:111 注释承诺）**：
/// 只呈现「上次扫描后没再见过」的文件，刚扫到的文件不会进幽灵清单。
#[tauri::command]
pub async fn preview_ghosts(state: State<'_, AppState>) -> Result<Vec<GhostItem>, String> {
    let db = state.db_conn.lock().map_err(|e| e.to_string())?;

    // 取最近一次合法完成的扫描批次的 started_at（journal.rs:111 注释承诺的过滤条件）。
    // 没有最近批次时传空串，SQL 的 `?1 = ''` 短路为真，等价于不过滤（兼容首次使用）。
    let recent_scan_started_at = journal::latest_completed_batch(&db, journal::BatchType::Scan)
        .and_then(|bid| {
            db.query_row(
                "SELECT started_at FROM batches WHERE batch_id = ?1",
                params![bid],
                |row| row.get::<_, String>(0),
            )
            .ok()
        })
        .unwrap_or_default();

    // SQL 加 last_seen_at 过滤（兑现 journal.rs 注释）：
    //   is_dead = 1 的文件全部返回（需要用户决定 restore/purge）；
    //   is_dead = 0 的文件只在「从未见过 OR 上次见到的时间早于最近一次扫描」时返回
    //   （刚扫到的文件不会进幽灵清单，避免误伤正常文件）。
    let mut stmt = db
        .prepare(
            "SELECT f.path, f.observed_mtime, f.observed_size, f.is_dead, f.dead_at, f.last_seen_at,
                    (SELECT COUNT(*) FROM frame_vectors fv WHERE fv.path = f.path) AS frame_count
             FROM files f
             WHERE f.is_dead = 1
                OR (f.is_dead = 0
                    AND (?1 = '' OR f.last_seen_at IS NULL OR f.last_seen_at < ?1))
             ORDER BY f.last_seen_at DESC",
        )
        .map_err(|e| format!("Prepare error: {}", e))?;

    let raw_items: Vec<GhostItem> = stmt
        .query_map(params![recent_scan_started_at], |row| {
            let path: String = row.get(0)?;
            Ok(GhostItem {
                path,
                frame_count: row.get::<_, i64>(6)? as usize,
                observed_mtime: row.get(1)?,
                observed_size: row.get(2)?,
                is_dead: row.get::<_, i64>(3)? != 0,
                dead_at: row.get(4)?,
                last_seen_at: row.get(5)?,
                // 默认值，下面按盘根可达性覆写
                exists_on_disk: false,
                disk_offline: false,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(Result::ok)
        .collect();
    drop(stmt);

    // 按盘根聚合做可达性检测：每个盘根只 `Path::exists()` 一次，
    // 避免对离线盘上的每个文件都做一次注定失败的系统调用。
    let mut root_reachable: std::collections::HashMap<String, bool> = std::collections::HashMap::new();

    let mut items: Vec<GhostItem> = Vec::with_capacity(raw_items.len());
    for mut item in raw_items {
        if item.is_dead {
            // 已标记失效：向量在库里，restore/purge 都不依赖磁盘，直接返回。
            // 不需要做 exists 检测（即便盘离线，purge 删的是库里行而非磁盘文件）。
            item.exists_on_disk = Path::new(&item.path).exists();
            item.disk_offline = false;
            items.push(item);
            continue;
        }

        // is_dead = 0：判断是「真消失了」还是「盘根离线」。
        let root = extract_root(&item.path);
        let reachable = *root_reachable
            .entry(root.clone())
            .or_insert_with(|| Path::new(&root).exists());

        if !reachable {
            // 盘根不可达：NAS 未挂载 / 移动硬盘未插。
            // 标 disk_offline = true，前端整组灰显并禁用 purge（P0-1 核心）。
            // 不做逐文件 exists 检测（盘根都不可达，逐文件检查必然都是 false）。
            item.exists_on_disk = false;
            item.disk_offline = true;
        } else {
            // 盘根可达：逐文件 exists 检测，判断该文件是否真的从磁盘上消失了。
            item.exists_on_disk = Path::new(&item.path).exists();
            item.disk_offline = false;
        }

        // 只呈现「需要用户拿主意」的：磁盘已不在（且盘根可达，确认是真消失），
        // 或已被标记失效。盘根离线的文件保留在列表里（让用户看到「⚠️ 该盘离线」），
        // 但前端会灰显并禁用 purge。
        items.push(item);
    }

    items.sort_by(|a, b| {
        // 离线盘的排到最后（最不需要立刻处理的），其次磁盘已不在的，其次按路径稳定排序
        a.disk_offline
            .cmp(&b.disk_offline)
            .then_with(|| a.exists_on_disk.cmp(&b.exists_on_disk))
            .then_with(|| a.path.cmp(&b.path))
    });

    fs_trace!(
        "ghost preview, candidates={}, dead={}, disk_offline={}",
        items.len(),
        items.iter().filter(|i| i.is_dead).count(),
        items.iter().filter(|i| i.disk_offline).count()
    );
    Ok(items)
}

/// 对指定路径执行幽灵处理动作。
///
/// - `mark_dead`：标记失效（向量保留，可回库）
/// - `restore`  ：回库（仅对已失效的路径有效）
/// - `purge`    ：真删除（不可回退）
#[tauri::command]
pub async fn apply_ghost_action(
    state: State<'_, AppState>,
    paths: Vec<String>,
    action: String,
) -> Result<GhostActionResult, String> {
    let action = GhostAction::from_str(&action)
        .ok_or_else(|| format!("Unknown ghost action: {}", action))?;

    if paths.is_empty() {
        return Err("No paths selected".to_string());
    }

    // 声明为 mut：purge 级联删除需要开事务（db.transaction() 要求 &mut self）。
    let mut db = state.db_conn.lock().map_err(|e| e.to_string())?;
    let mut memory = state.memory_db.write().map_err(|e| e.to_string())?;

    // 开一个 clean_ghosts 批次骨架：这次批量操作动了什么，事后一查便知。
    let batch_id = journal::begin_batch(&db, BatchType::CleanGhosts, None)
        .unwrap_or_else(|e| format!("clean_ghosts_fallback_{}", e));

    let mut affected: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut affected_frames = 0usize;

    for path in &paths {
        let is_dead: Option<i64> = db
            .query_row(
                "SELECT is_dead FROM files WHERE path = ?1",
                params![path],
                |row| row.get(0),
            )
            .ok();

        let frame_count: usize = db
            .query_row(
                "SELECT COUNT(*) FROM frame_vectors WHERE path = ?1",
                params![path],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0) as usize;

        // NAS 守卫在行动侧的兜底（P0-1）：purge 离线盘上的文件会误删只是暂时离线的数据。
        // 预览侧已标 disk_offline，但用户可能在预览后盘才离线，行动侧再检查一次。
        if action == GhostAction::Purge && is_dead != Some(1) {
            let root = extract_root(path);
            if !Path::new(&root).exists() {
                skipped.push(path.clone());
                continue;
            }
            // 盘根可达，再确认文件确实不在（双重保险）。
            if Path::new(path).exists() {
                // 文件又出现了（盘重新挂载后），不该 purge。
                skipped.push(path.clone());
                continue;
            }
        }

        match action {
            GhostAction::MarkDead => {
                if is_dead == Some(1) {
                    skipped.push(path.clone());
                    continue;
                }
                db.execute(
                    "UPDATE files SET is_dead = 1, dead_at = ?1 WHERE path = ?2",
                    params![utc_now_iso(), path],
                )
                .map_err(|e| e.to_string())?;
                let _ = journal::record_event(&db, &batch_id, path, EventType::Dead, None);
            }
            GhostAction::Restore => {
                if is_dead != Some(1) {
                    skipped.push(path.clone());
                    continue;
                }
                db.execute(
                    "UPDATE files SET is_dead = 0, dead_at = NULL WHERE path = ?1",
                    params![path],
                )
                .map_err(|e| e.to_string())?;
                let _ = journal::record_event(&db, &batch_id, path, EventType::Revived, None);
            }
            GhostAction::Purge => {
                // **级联删除关联表（P0-2 / 第三轮 D4-D5）**：
                // 旧实现只删 frame_vectors + files，留下 media_ocr_entries /
                // media_text_vectors / reindex_backup 三张表的孤儿行，
                // 配合 search_text 不过滤 is_dead ⇒ 真删除后仍能被文本搜出来。
                // 这是品牌级失信（"100% 本地、数据主权" + "删不掉"）。
                let tx = db.transaction().map_err(|e| e.to_string())?;
                tx.execute("DELETE FROM frame_vectors WHERE path = ?1", params![path])
                    .map_err(|e| e.to_string())?;
                tx.execute("DELETE FROM media_ocr_entries WHERE path = ?1", params![path])
                    .map_err(|e| e.to_string())?;
                tx.execute("DELETE FROM media_text_vectors WHERE path = ?1", params![path])
                    .map_err(|e| e.to_string())?;
                // 债单 B15：补删 text_entries 中以该路径为 source_uri 的文本块，
                // 否则 purge 后这些块仍可被 search_text_entries 召回（同类于第三轮 D4，换张表）。
                tx.execute("DELETE FROM text_entries WHERE source_uri = ?1", params![path])
                    .map_err(|e| e.to_string())?;
                tx.execute("DELETE FROM reindex_backup WHERE path = ?1", params![path])
                    .map_err(|e| e.to_string())?;
                tx.execute("DELETE FROM files WHERE path = ?1", params![path])
                    .map_err(|e| e.to_string())?;
                tx.commit().map_err(|e| e.to_string())?;
                let _ = journal::record_event(
                    &db,
                    &batch_id,
                    path,
                    EventType::Purged,
                    Some(&json!({ "frame_count": frame_count }).to_string()),
                );
            }
        }

        affected.push(path.clone());
        affected_frames += frame_count;
    }

    // 内存矩阵同步：
    //  mark_dead / purge → 移出矩阵；restore → 从库里把向量读回来。
    match action {
        GhostAction::Restore => {
            // 回库：向量一直原样躺在库里，直接读回内存即可，无需重新编码。
            let rows = load_frame_rows(&db, Some(&affected));
            for row in rows.rows {
                memory.push(
                    row.path,
                    row.timestamp,
                    row.vector,
                    row.ocr_text,
                    row.user_note,
                    row.index_time,
                );
            }
        }
        GhostAction::MarkDead | GhostAction::Purge => {
            memory.remove_by_paths(&affected);
        }
    }

    let summary = json!({
        "action": action.as_str(),
        "affected": affected.len(),
        "affected_frames": affected_frames,
        "skipped": skipped.clone(),
    })
    .to_string();
    if let Err(e) = journal::finish_batch(&db, &batch_id, &summary) {
        println!("⚠️ Failed to close clean_ghosts batch {}: {}", batch_id, e);
    }

    fs_trace!(
        "ghost action applied, batch_id={}, action={}, affected={}, frames={}, skipped={}",
        batch_id,
        action.as_str(),
        affected.len(),
        affected_frames,
        skipped.len()
    );

    Ok(GhostActionResult {
        batch_id,
        action: action.as_str().to_string(),
        affected_paths: affected,
        affected_frames,
        skipped_paths: skipped,
    })
}
