// SQLite 初始化、WAL 配置、schema 迁移以及向量行的读取。

use std::fs;
use std::time::Instant;

use rusqlite::{params, Connection};

use crate::constants::VECTOR_DIM;
use crate::fs_trace;
use crate::model_code::StartupCleanupReport;
use super::app_settings;
use super::journal;
use super::migrations;
use super::vector_blob::decode_f32_le_checked;
use super::vector_matrix::FlatVectorMatrix;

/// 默认的事件保留天数（与旧行为一致）。`-1` = 永不清理。
const DEFAULT_EVENT_RETENTION_DAYS: i64 = 90;

/// M2''' 备份 restore：打开数据库前，应用 pending 恢复标记（若存在）。
///
/// 恢复的难点在于：持活连接时覆盖 `.db` 会损坏数据库，故采用「pending restore」——
/// `restore_backup` 命令只写标记文件，真正替换在下次启动、连接建立之前完成。
fn apply_pending_restore(app_data_dir: &std::path::Path, db_path: &std::path::Path) {
    let marker = app_data_dir.join("pending_restore.txt");
    if !marker.exists() {
        return;
    }
    let backup_path = match fs::read_to_string(&marker) {
        Ok(s) => s.trim().to_string(),
        Err(_) => {
            let _ = fs::remove_file(&marker);
            return;
        }
    };
    let backup = std::path::PathBuf::from(&backup_path);
    if backup.exists() {
        // 用备份覆盖主库；同时清掉旧 WAL/SHM，让恢复后的库从干净状态开始。
        match fs::copy(&backup, db_path) {
            Ok(_) => {
                let wal = db_path.with_extension("db-wal");
                let shm = db_path.with_extension("db-shm");
                let _ = fs::remove_file(&wal);
                let _ = fs::remove_file(&shm);
                println!("♻️ Database restored from backup: {}", backup_path);
            }
            Err(e) => println!("⚠️ Failed to restore from backup {}: {}", backup_path, e),
        }
    } else {
        println!("⚠️ Pending restore backup no longer exists: {}", backup_path);
    }
    let _ = fs::remove_file(&marker);
}

pub fn init_db_and_load_memory() -> (Connection, FlatVectorMatrix, StartupCleanupReport) {
    println!("💾 Connecting to local SQLite Hybrid Matrix...");
    let app_data_dir = dirs::data_local_dir().unwrap().join("FrameScout-Offline_AI_Search-Global");
    if let Err(e) = fs::create_dir_all(&app_data_dir) {
        println!("⚠️ Warning: Failed to create AppData directory: {}", e);
    }

    let db_path = app_data_dir.join("framescout-offline_ai_search-global.db");

    // M2''' 备份 restore：pending 恢复标记——打开数据库前，若用户标记了「恢复某备份」，
    // 先用该备份替换主库文件（此时连接尚未建立，.db 未被锁，可安全覆盖）。
    apply_pending_restore(&app_data_dir, &db_path);

    // 声明为 mut：迁移链需要开启事务（&mut self）。
    let mut conn = Connection::open(&db_path).expect("Failed to open database");

    // WAL mode allows concurrent reads during writes（边建边搜的前提：
    // 后台批量写入时，主进程的读查询不会被锁住）。
    // busy_timeout 让并发写等待 5s 而非立刻抛 `database is locked`（P0-5 / 第三轮 D1）：
    // 旧的 synchronous=NORMAL + 无 busy_timeout 在偶发并发写时直接失败。
    if let Err(e) = conn.execute_batch(
        "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA busy_timeout = 5000;",
    ) {
        println!("⚠️ Warning: Failed to apply WAL pragmas: {}", e);
    }

    // 版本化迁移链：负责建表、历史库升级，以及迁移前的备份。
    // 版本一致时零 IO；仅当代码期望版本 > 库当前版本时才备份并迁移。
    migrations::run_migrations(&mut conn, &db_path);

    // 把上次崩溃留下的半截批次标为 interrupted。它们不算合法完成批次，
    // 因此不会成为差异报告的骨架（合法判定固定为 finished_at IS NOT NULL）。
    match journal::mark_interrupted_batches(&conn) {
        Ok(0) => {}
        Ok(n) => println!("⚠️ Marked {} interrupted batch(es) from a previous crash.", n),
        Err(e) => println!("⚠️ Failed to mark interrupted batches: {}", e),
    }

    // M0'''：读取用户的自动清理策略（事件保留天数 / 备份份数 / 备份天数）。
    // 「永不 / 不限」用哨兵 -1 表示。缺失时回退旧行为默认值。
    let event_retention_days = app_settings::get_i64(
        &conn,
        app_settings::KEY_EVENT_RETENTION_DAYS,
        DEFAULT_EVENT_RETENTION_DAYS,
    );
    let backup_keep = app_settings::get_i64(
        &conn,
        app_settings::KEY_BACKUP_KEEP_COUNT,
        journal::BACKUP_KEEP_PER_PATH as i64,
    );
    let backup_max_age_days = app_settings::get_i64(
        &conn,
        app_settings::KEY_BACKUP_MAX_AGE_DAYS,
        journal::BACKUP_MAX_AGE_DAYS,
    );

    let mut cleanup = StartupCleanupReport::default();

    // 重索引冷备份的裁剪（按路径留最近 N 份、清掉 N 天前的）。
    // 冷备份是保险而非档案馆，不能无限膨胀。份数/天数由用户可设（-1 = 不限）。
    match journal::prune_reindex_backups(&conn, backup_keep, backup_max_age_days) {
        Ok(0) => {}
        Ok(n) => {
            cleanup.pruned_reindex_backups = n;
            fs_trace!("pruned {} stale reindex backup row(s)", n);
        }
        Err(e) => println!("⚠️ Failed to prune reindex backups: {}", e),
    }

    // P1-15 / D11：file_events / batches 裁剪——旧实现只增不减，全仓无任何裁剪语句。
    // 保留最近 N 天的事件记录，超出的清理（冷备份式清理，不影响数据主权核心表）。
    // M0'''：N 由用户设置（-1 = 永不清理）。
    match prune_old_events_and_batches(&conn, event_retention_days) {
        Ok((events, batches)) => {
            cleanup.pruned_events = events;
            cleanup.pruned_batches = batches;
            if events > 0 || batches > 0 {
                fs_trace!("pruned {} old file_events, {} old batches", events, batches);
            }
        }
        Err(e) => println!("⚠️ Failed to prune old events/batches: {}", e),
    }

    // 最近一次合法完成的扫描批次，是差异报告的骨架来源
    // （合法判定固定为 finished_at IS NOT NULL，崩溃留下的半截批次不算）。
    if let Some(last_scan) = journal::latest_completed_batch(&conn, journal::BatchType::Scan) {
        fs_trace!("latest completed scan batch: {}", last_scan);
    }

    let started = Instant::now();
    let loaded_rows = load_frame_rows(&conn, None);

    let mut memory_matrix = FlatVectorMatrix::new(VECTOR_DIM);
    for row in loaded_rows.rows {
        memory_matrix.push(
            row.path,
            row.timestamp,
            row.vector,
            row.ocr_text,
            row.user_note,
            row.index_time,
        );
    }
    let loaded = memory_matrix.len();

    // 清理既无有效 BLOB、也无有效 JSON 的残行。
    // 注意：复合主键下"按 path 删除"会删掉该文件的全部帧，符合"整文件维度过期即清理"的语义。
    // M0'''：清理数量计入 StartupCleanupReport，前端启动后 Toast 告知。
    let stale_count = loaded_rows.stale_paths.len();
    if stale_count > 0 {
        println!("🧹 Cleaning up {} obsolete database records...", stale_count);
        for path in &loaded_rows.stale_paths {
            let _ = conn.execute("DELETE FROM frame_vectors WHERE path = ?1", params![path]);
        }
        cleanup.cleaned_residual_rows = stale_count;
    }

    // 第八轮：给这笔清理账盖时间戳。没有它，设置页只能说「清了什么」，
    // 说不出「什么时候清的」—— 不可逆的删除连时间都查不到，就不是账，是传言。
    if cleanup.pruned_events + cleanup.pruned_batches + cleanup.pruned_reindex_backups
        + cleanup.cleaned_residual_rows
        > 0
    {
        cleanup.cleaned_at = Some(crate::time_util::utc_now_iso());
    }

    println!(
        "✅ Memory matrix loaded! Holding {} spatio-temporal slices (Dim: {}).",
        memory_matrix.len(),
        VECTOR_DIM
    );
    fs_trace!(
        "db loaded, loaded={}, from_blob={}, from_json_legacy={}, stale_cleaned={}, took={:?}, schema_version={}",
        loaded,
        loaded.saturating_sub(loaded_rows.json_fallback),
        loaded_rows.json_fallback,
        stale_count,
        started.elapsed(),
        migrations::current_schema_version(&conn)
    );

    (conn, memory_matrix, cleanup)
}

// =========================================================================
//  向量行读取
// =========================================================================

/// 一条解码完成的帧记录。
pub struct FrameRow {
    pub path: String,
    pub timestamp: f32,
    pub vector: Vec<f32>,
    pub ocr_text: String,
    pub user_note: String,
    pub index_time: f64,
}

/// `load_frame_rows` 的结果。
#[derive(Default)]
pub struct LoadedRows {
    /// 可正常使用的行
    pub rows: Vec<FrameRow>,
    /// BLOB 与 JSON 都解不出有效向量的路径（维度不符或两者皆空）
    pub stale_paths: Vec<String>,
    /// 其中有多少行是走 JSON 回退读出来的（存量转换进度指示）
    pub json_fallback: usize,
}

/// 读取并解码 `frame_vectors`。启动载入与「回库」共用同一段解码逻辑，
/// 避免两处各写一份 BLOB/JSON 优先级判断而日后漂移。
///
/// - `only_paths = None`：读取全部未失效的行（启动用）
/// - `only_paths = Some(paths)`：只读这些路径（复活失效文件后补回内存用）
///
/// **files.is_dead 与搜索矩阵的边界（写死）**：标记失效后其 `frame_vectors` 行
/// **保留**，但不再参与矩阵构建；复活时直接恢复参与，无需重新编码。
/// 用 LEFT JOIN 而非 INNER JOIN：万一某行还没有 `files` 锚点（理论不该发生），
/// 仍然照常返回，绝不因为账簿缺失而丢数据。
pub fn load_frame_rows(conn: &Connection, only_paths: Option<&[String]>) -> LoadedRows {
    let mut out = LoadedRows::default();

    let sql = match only_paths {
        None => "SELECT fv.path, fv.timestamp, fv.vector_f32, fv.vector_json, fv.ocr_text, fv.user_note, fv.index_time
                 FROM frame_vectors fv
                 LEFT JOIN files f ON f.path = fv.path
                 WHERE f.is_dead IS NULL OR f.is_dead = 0
                 ORDER BY fv.index_time DESC"
            .to_string(),
        Some(paths) if !paths.is_empty() => {
            let placeholders = std::iter::repeat_n("?", paths.len())
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "SELECT fv.path, fv.timestamp, fv.vector_f32, fv.vector_json, fv.ocr_text, fv.user_note, fv.index_time
                 FROM frame_vectors fv
                 LEFT JOIN files f ON f.path = fv.path
                 WHERE (f.is_dead IS NULL OR f.is_dead = 0) AND fv.path IN ({})
                 ORDER BY fv.index_time DESC",
                placeholders
            )
        }
        Some(_) => return out,
    };

    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => {
            println!("⚠️ Failed to prepare frame_vectors query: {}", e);
            return out;
        }
    };

    let params: Vec<String> = only_paths.unwrap_or(&[]).to_vec();
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        let path: String = row.get(0)?;
        let ts: f64 = row.get(1)?;
        let blob: Option<Vec<u8>> = row.get(2)?;
        let json_str: Option<String> = row.get(3)?;
        let ocr_text: String = row.get(4)?;
        let user_note: String = row.get(5).unwrap_or_default();
        let index_time: f64 = row.get(6)?;

        // 读取策略：**优先 BLOB，旧行回退 JSON**。
        // 新写入只填 vector_f32；v3.2 之前的存量行只有 vector_json，
        // 该列保留的唯一意义就是让这些旧行仍可读。
        let mut from_json = false;
        let vector: Vec<f32> = match blob.as_deref().and_then(|b| decode_f32_le_checked(b, VECTOR_DIM)) {
            Some(v) => v,
            None => {
                from_json = true;
                json_str
                    .as_deref()
                    .and_then(|s| serde_json::from_str::<Vec<f32>>(s).ok())
                    .unwrap_or_default()
            }
        };
        Ok((path, ts as f32, vector, ocr_text, user_note, index_time, from_json))
    });

    if let Ok(rows) = rows {
        for row in rows.flatten() {
            let (path, timestamp, vector, ocr_text, user_note, index_time, from_json) = row;
            if vector.len() != VECTOR_DIM {
                println!("⚠️ Found obsolete vector (dim: {}) for path: {}. Marking for clean.", vector.len(), path);
                out.stale_paths.push(path);
                continue;
            }
            if from_json {
                out.json_fallback += 1;
            }
            out.rows.push(FrameRow {
                path,
                timestamp,
                vector,
                ocr_text,
                user_note,
                index_time,
            });
        }
    }

    out
}

/// P1-15 / D11：裁剪过期的 file_events 和 batches。
///
/// 旧实现：`file_events` / `batches` 只增不减，全仓 grep 无任何 DELETE 语句，
/// 只有 `journal.rs` 裁了 `reindex_backup`。长期使用后这两张表会无限膨胀。
///
/// M0''' 第 1 项：保留天数 `retention_days` 由用户设置（`app_settings`）传入，
/// `-1` 表示「永不清理」——此时跳过裁剪，直接返回 (0, 0)。
fn prune_old_events_and_batches(
    conn: &Connection,
    retention_days: i64,
) -> Result<(usize, usize), rusqlite::Error> {
    // 永不清理：不做任何裁剪。
    if retention_days < 0 {
        return Ok((0, 0));
    }

    // retention_days 天前的 Unix 时间戳（秒）
    let cutoff = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64)
        - retention_days * 24 * 3600;

    let events = conn.execute(
        "DELETE FROM file_events WHERE occurred_at < ?1 AND event_type IN ('purged', 'reindex_failed', 'content_modified')",
        params![cutoff],
    )?;

    let batches = conn.execute(
        "DELETE FROM batches WHERE finished_at IS NOT NULL AND started_at < ?1",
        params![cutoff],
    )?;

    Ok((events, batches))
}
