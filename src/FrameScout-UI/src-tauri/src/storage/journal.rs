// =========================================================================
//  账簿层：files（状态锚点） / batches（批次骨架） / file_events（生命日记）
// =========================================================================
//  三张表各司其职、互相引用、不可互相替代：
//   - `files`       —— **只保留最新状态**：文件级生命周期锚点（在不在、多大、何时改过）
//   - `batches`     —— **只追加**：每次批量操作的骨架（扫描 / 幽灵清理 / 重索引 / 存量导入）
//   - `file_events` —— **只追加、永不更新**：世界是如何变成现在这样的
//
//  设计活动中心 / 时间轴时：从 `batches` 查骨架、从 `file_events` 查血肉、
//  从 `files` 查快照。边界清晰即无人迷路。
//
//  写入纪律：
//   - `files.last_seen_at` = 系统最后一次**确认该文件仍在磁盘**的时刻。
//     凡扫描/索引批次中确认存在且未失效，就刷新它。
//   - `observed_change` 是**纯客观感知信号**：云同步、批量 touch 也会触发，
//     不代表内容真的变了。用户决策（忽略/重索引）绝不回写 `observed_*`。
//   - `content_modified` 只在**重索引真正成功完成后**写入（见差异报告行动层），
//     本文件不提供该写入口，避免账簿撒谎。

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};

use crate::time_util::{utc_now_iso, utc_stamp};

/// mtime 差异的宽容阈值（毫秒）。
///
/// Windows NTFS 精度 100ns、FAT/exFAT 2s、SMB 1s——跨文件系统拷贝或云同步后，
/// mtime 会因精度损失而漂移。小于该阈值的差异一律视为「未修改」，
/// 把误差留给文件系统，而不是拿噪音污染事件表。
///
/// 已知代价（假阴性）：文件恰在 2 秒内被真实修改、且 size 恰好未变时会被漏报，
/// 概率极小但非零。缓解手段是 `observed_size` 作为第二信号。
pub const MTIME_TOLERANCE_MS: i64 = 2000;

// =========================================================================
//  文件观察状态
// =========================================================================

/// 磁盘上读到的文件指纹。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileStat {
    /// 修改时间，epoch 毫秒
    pub mtime_ms: i64,
    /// 字节大小
    pub size: i64,
}

/// 读取文件指纹。文件不存在或无权限时返回 None（不视为错误——
/// 「看不到」本身就是要记录的一种状态，交由调用方决定如何处理）。
pub fn stat_file(path: &str) -> Option<FileStat> {
    let meta = fs::metadata(path).ok()?;
    let mtime_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)?;
    Some(FileStat {
        mtime_ms,
        size: meta.len() as i64,
    })
}

/// `upsert_file_observed` 的判定结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Observation {
    /// 首次进入索引
    FirstSeen,
    /// mtime 或 size 相对上次观察发生了变化（已过 2s 宽容阈值）
    Changed,
    /// 与上次观察一致
    Unchanged,
}

/// 变化判定（纯函数，抽出来只为可测——阈值边界是全账簿最容易出错的地方）。
///
/// 规则：
///  - 这次没看到文件 → Unchanged（"看不到"不改变既有观察值，交由调用方决定）
///  - 旧值为 NULL（首次补基线时没 stat 到）→ 视为 Changed，补上真实观察值
///  - mtime 差异 <= MTIME_TOLERANCE_MS 视为未修改（留给文件系统精度差）
///  - size 不等 → Changed（第二信号，弥补 2s 阈值内的漏报盲区）
pub fn classify_change(
    prev_mtime: Option<i64>,
    prev_size: Option<i64>,
    new: Option<FileStat>,
) -> Observation {
    let new = match new {
        Some(s) => s,
        None => return Observation::Unchanged,
    };

    let mtime_changed = match prev_mtime {
        Some(old) => (old - new.mtime_ms).abs() > MTIME_TOLERANCE_MS,
        None => true,
    };
    let size_changed = match prev_size {
        Some(old) => old != new.size,
        None => true,
    };

    if mtime_changed || size_changed {
        Observation::Changed
    } else {
        Observation::Unchanged
    }
}

/// 记录一次「该文件此刻仍存在于磁盘」的观察，并刷新 `last_seen_at`。
///
/// 语义（写死）：**只要确认存在且未失效，就刷新 last_seen_at**。
/// 幽灵清理因此只需筛 `is_dead = 0 AND last_seen_at < 最近一次合法扫描批次`，
/// 就能找出「不知是否还在」的文件，不会误伤正常文件。
pub fn upsert_file_observed(
    conn: &Connection,
    path: &str,
    stat: Option<FileStat>,
) -> Result<Observation, String> {
    // 先读旧值，判定 first_seen / changed / unchanged。
    let prev: Option<(Option<i64>, Option<i64>)> = conn
        .query_row(
            "SELECT observed_mtime, observed_size FROM files WHERE path = ?1",
            params![path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    let now = utc_now_iso();

    let observation = match prev {
        None => Observation::FirstSeen,
        Some((old_mtime, old_size)) => classify_change(old_mtime, old_size, stat),
    };

    match stat {
        Some(s) => {
            // 看到了文件：刷新观察值与 last_seen_at。
            conn.execute(
                "INSERT INTO files (path, observed_mtime, observed_size, is_dead, first_seen_at, last_seen_at)
                 VALUES (?1, ?2, ?3, 0, ?4, ?4)
                 ON CONFLICT(path) DO UPDATE SET
                    observed_mtime = excluded.observed_mtime,
                    observed_size  = excluded.observed_size,
                    last_seen_at   = excluded.last_seen_at",
                params![path, s.mtime_ms, s.size, now],
            )
            .map_err(|e| format!("upsert files failed: {}", e))?;
        }
        None => {
            // 这次没看到文件：只建/保留锚点，绝不改写 observed_* 与 last_seen_at
            // （"看不到"不等于"不存在"，可能是权限或暂时性离线）。
            conn.execute(
                "INSERT INTO files (path, is_dead, first_seen_at, last_seen_at)
                 VALUES (?1, 0, ?2, ?2)
                 ON CONFLICT(path) DO NOTHING",
                params![path, now],
            )
            .map_err(|e| format!("insert files failed: {}", e))?;
        }
    }

    Ok(observation)
}

// =========================================================================
//  重索引冷备份
// =========================================================================

/// 每个路径保留的备份份数上限。
pub const BACKUP_KEEP_PER_PATH: u32 = 3;
/// 备份保留天数上限。
pub const BACKUP_MAX_AGE_DAYS: i64 = 30;

/// 在**同一事务内**、执行 DELETE 之前，把某路径当前的帧行整批拷进冷备份表。
///
/// 必须与被删行同事务：要么「旧向量已备份且被替换」，要么「什么都没发生」，
/// 绝不留下「旧的删了、备份也没存上」的中间态。
pub fn snapshot_frames_before_replace(
    conn: &Connection,
    path: &str,
    backup_batch_id: &str,
) -> Result<usize, String> {
    conn.execute(
        "INSERT INTO reindex_backup
            (path, timestamp, vector_f32, vector_json, ocr_text, user_note, index_time, backup_batch_id, created_at)
         SELECT path, timestamp, vector_f32, vector_json, ocr_text, user_note, index_time, ?1, ?2
         FROM frame_vectors WHERE path = ?3",
        params![backup_batch_id, utc_now_iso(), path],
    )
    .map_err(|e| format!("snapshot frames failed: {}", e))
}

/// 启动时的备份裁剪：按路径只留最近 `BACKUP_KEEP_PER_PATH` 份，
/// 并清掉超过 `BACKUP_MAX_AGE_DAYS` 天的。与 `file_events` 的裁剪思路一致——
/// 冷备份是保险，不是历史档案馆，不能无限膨胀。
pub fn prune_reindex_backups(conn: &Connection) -> Result<usize, String> {
    let keep = BACKUP_KEEP_PER_PATH;
    // 每个路径按 created_at 倒序编号，留下前 keep 份
    let by_count = conn
        .execute(
            "DELETE FROM reindex_backup
             WHERE id IN (
                 SELECT id FROM (
                     SELECT id, ROW_NUMBER() OVER (PARTITION BY path ORDER BY created_at DESC, id DESC) AS rn
                     FROM reindex_backup
                 ) WHERE rn > ?1
             )",
            params![keep],
        )
        .map_err(|e| format!("prune backups by count failed: {}", e))?;

    let cutoff = cutoff_iso(BACKUP_MAX_AGE_DAYS);
    let by_age = conn
        .execute(
            "DELETE FROM reindex_backup WHERE created_at < ?1",
            params![cutoff],
        )
        .map_err(|e| format!("prune backups by age failed: {}", e))?;

    Ok(by_count + by_age)
}

/// N 天前的 ISO 时间戳（字典序可直接与 `created_at` 比较）。
fn cutoff_iso(days: i64) -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
        - days * 86400;
    let clamped = if secs < 0 { 0 } else { secs as u64 };
    let (y, m, d) = crate::time_util::civil_from_days((clamped / 86400) as i64);
    let hms = clamped % 86400;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        hms / 3600,
        (hms % 3600) / 60,
        hms % 60
    )
}

// =========================================================================
//  事件
// =========================================================================

/// 事件类型（只追加，不修改语义）。
///
/// 目前只有 `FirstSeen` / `ObservedChange` 有写入口（扫描循环）。
/// `ContentModified` / `ReindexFailed` 由差异报告的「重新索引」动作写入（因果链：
/// 发现差异 → 用户确认 → 成功才写 content_modified，失败写 reindex_failed），
/// `Dead` / `Revived` 由幽灵清理的「标记失效 / 复活」写入——均在下一批接入。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// 路径首次进入索引
    FirstSeen,
    /// 扫描发现 mtime/size 变化（纯客观记录，不代表内容真变）
    ObservedChange,
    /// 重索引**成功完成后**才写，payload 记 old/new frame count
    ContentModified,
    /// 重索引失败，文件落入差异报告第四组待重试
    ReindexFailed,
    /// 标记失效（向量保留，可复活）
    Dead,
    /// 复活（无需重新编码）
    Revived,
    /// 真删除：连同向量行一并移除。**终态，不可回退**，
    /// 因此「整批回滚」不适用于它——回滚只覆盖状态类事件。
    Purged,
}

impl EventType {
    pub fn as_str(self) -> &'static str {
        match self {
            EventType::FirstSeen => "first_seen",
            EventType::ObservedChange => "observed_change",
            EventType::ContentModified => "content_modified",
            EventType::ReindexFailed => "reindex_failed",
            EventType::Dead => "dead",
            EventType::Revived => "revived",
            EventType::Purged => "purged",
        }
    }
}

/// 追加一条事件。**只追加，永不更新**——这是账簿可信的前提。
pub fn record_event(
    conn: &Connection,
    batch_id: &str,
    path: &str,
    event_type: EventType,
    payload_json: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO file_events (batch_id, path, event_type, occurred_at, payload_json)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![batch_id, path, event_type.as_str(), utc_now_iso(), payload_json],
    )
    .map_err(|e| format!("record event failed: {}", e))?;
    Ok(())
}

// =========================================================================
//  批次
// =========================================================================

/// 批次类型。所有批量操作共用 `batches` 这一张骨架表，
/// 因此「整批回滚 / 来源筛选」只需 `WHERE batch_id = ?`，无需额外表结构。
///
/// 目前只有 `Scan` 与 `LegacyImport` 有写入口，其余随对应功能接入。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchType {
    Scan,
    CleanGhosts,
    Reindex,
    LegacyImport,
}

impl BatchType {
    pub fn as_str(self) -> &'static str {
        match self {
            BatchType::Scan => "scan",
            BatchType::CleanGhosts => "clean_ghosts",
            BatchType::Reindex => "reindex",
            BatchType::LegacyImport => "legacy_import",
        }
    }
}

/// 开一个批次，返回其 `batch_id`（`scan_20261002_103747` 形态）。
/// 此时 `status = running`、`finished_at` 为空。
pub fn begin_batch(
    conn: &Connection,
    batch_type: BatchType,
    id_suffix: Option<&str>,
) -> Result<String, String> {
    let batch_id = match id_suffix {
        Some(s) => format!("{}_{}", batch_type.as_str(), s),
        None => format!("{}_{}", batch_type.as_str(), utc_stamp()),
    };
    conn.execute(
        "INSERT OR IGNORE INTO batches (batch_id, batch_type, started_at, status)
         VALUES (?1, ?2, ?3, 'running')",
        params![batch_id, batch_type.as_str(), utc_now_iso()],
    )
    .map_err(|e| format!("begin batch failed: {}", e))?;
    Ok(batch_id)
}

/// 正常结束一个批次：写 `finished_at` 与 `summary_json`，状态置 completed。
///
/// **批次合法判定（写死）**：`finished_at IS NOT NULL` 才算合法完成。
/// 中途崩溃留下的半截批次不参与差异报告骨架。
pub fn finish_batch(conn: &Connection, batch_id: &str, summary_json: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE batches SET finished_at = ?1, status = 'completed', summary_json = ?2
         WHERE batch_id = ?3",
        params![utc_now_iso(), summary_json, batch_id],
    )
    .map_err(|e| format!("finish batch failed: {}", e))?;
    Ok(())
}

/// 被取消的批次收尾：写 `finished_at` 与 `summary_json`，状态置 `cancelled`（债单 C5）。
///
/// 与正常 `completed` 区分开，让扫描报告能区分「被用户中止」与「正常完成」。
/// `batches.status` 是 TEXT 列，无需改表结构，向后兼容。
pub fn finish_batch_cancelled(conn: &Connection, batch_id: &str, summary_json: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE batches SET finished_at = ?1, status = 'cancelled', summary_json = ?2
         WHERE batch_id = ?3",
        params![utc_now_iso(), summary_json, batch_id],
    )
    .map_err(|e| format!("finish cancelled batch failed: {}", e))?;
    Ok(())
}

/// 启动时把上次崩溃留下的半截批次标记为 interrupted。
/// 返回被标记的数量。
pub fn mark_interrupted_batches(conn: &Connection) -> Result<usize, String> {
    let n = conn
        .execute(
            "UPDATE batches SET status = 'interrupted'
             WHERE status = 'running' AND finished_at IS NULL",
            [],
        )
        .map_err(|e| format!("mark interrupted batches failed: {}", e))?;
    Ok(n)
}

/// 取最近一次**合法完成**的批次 id（供差异报告做骨架）。
/// 固定条件：`finished_at IS NOT NULL ORDER BY id DESC LIMIT 1`。
pub fn latest_completed_batch(conn: &Connection, batch_type: BatchType) -> Option<String> {
    conn.query_row(
        "SELECT batch_id FROM batches
         WHERE batch_type = ?1 AND finished_at IS NOT NULL
         ORDER BY id DESC LIMIT 1",
        params![batch_type.as_str()],
        |row| row.get(0),
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::{classify_change, FileStat, MTIME_TOLERANCE_MS, Observation};

    fn stat(mtime_ms: i64, size: i64) -> Option<FileStat> {
        Some(FileStat { mtime_ms, size })
    }

    #[test]
    fn missing_file_does_not_disturb_existing_observation() {
        assert_eq!(classify_change(Some(1000), Some(10), None), Observation::Unchanged);
    }

    #[test]
    fn identical_observation_is_unchanged() {
        assert_eq!(
            classify_change(Some(1000), Some(10), stat(1000, 10)),
            Observation::Unchanged
        );
    }

    #[test]
    fn mtime_drift_within_tolerance_is_ignored() {
        // 恰好等于阈值：视为未修改（不误伤跨文件系统的精度漂移）
        assert_eq!(
            classify_change(Some(1000), Some(10), stat(1000 + MTIME_TOLERANCE_MS, 10)),
            Observation::Unchanged
        );
    }

    #[test]
    fn real_mtime_change_is_detected() {
        assert_eq!(
            classify_change(Some(1000), Some(10), stat(1000 + MTIME_TOLERANCE_MS + 1, 10)),
            Observation::Changed
        );
    }

    #[test]
    fn size_is_a_second_signal_inside_the_tolerance_window() {
        // 已知漏报盲区的缓解：mtime 在阈值内、但大小变了，仍判定为变化
        assert_eq!(
            classify_change(Some(1000), Some(10), stat(1000 + 500, 999)),
            Observation::Changed
        );
    }

    #[test]
    fn null_previous_observation_counts_as_change() {
        assert_eq!(classify_change(None, None, stat(1000, 10)), Observation::Changed);
    }
}
