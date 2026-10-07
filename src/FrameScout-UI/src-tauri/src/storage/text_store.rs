// 文本向量（BGE-M3 dense）的落库（检索表 `media_text_vectors` 的唯一写入口）。
//
// 语义（与战略规划书 5.3 / 协议 4.6 对齐）：
//   - `media_text_vectors` 存的是媒体**内嵌文字**（OCR 文本）的 BGE-M3 dense 向量，
//     属于媒体属性；纯文本条目（笔记/文档）走另一张 `text_entries` 表，两者分开。
//   - 本轮只落 dense，`sparse_json` 暂存 NULL——sparse 留给 FTS5 trigram 那批。
//   - `model` 固定 'bge-m3'，检索时严格按 model 过滤，绝不跨模型算相似度。
//   - `timestamp` 与 `media_ocr_entries` 对齐为**毫秒**（视频帧时间戳；图片恒为 0），
//     便于文本向量与 OCR 行按 (path, timestamp) 关联。

use rusqlite::{params, Connection};

use crate::constants::BGE_DENSE_DIM;
use crate::model_code::{TextEntryItem, TextEntryListResponse};
use super::vector_blob::encode_f32_le;

/// media_text_vectors.model 的取值（当前唯一模型）。
pub const BGE_MODEL_TAG: &str = "bge-m3";

/// 一条待落库的文本向量（对应 media_text_vectors 一行）。
pub struct TextVectorRow {
    /// 视频帧时间戳（毫秒）；图片恒为 0
    pub timestamp_ms: i64,
    /// BGE-M3 dense 向量（1024 维）
    pub dense: Vec<f32>,
}

/// 覆盖写入某文件的全部文本向量（先删后插）。
///
/// 必须与「帧向量写入」处在同一事务里调用，保证文本向量与视觉向量要么一起落库、
/// 要么一起回滚，不出现「视觉在但文本丢了」或反之的半截状态。
///
/// 维度校验：dense 非 1024 维（维度不符）的行直接跳过，绝不静默写入错维度数据。
pub fn replace_text_vectors(
    conn: &Connection,
    path: &str,
    rows: &[TextVectorRow],
) -> Result<(), String> {
    conn.execute("DELETE FROM media_text_vectors WHERE path = ?1", params![path])
        .map_err(|e| format!("delete text vectors failed: {}", e))?;

    for row in rows {
        if row.dense.len() != BGE_DENSE_DIM {
            println!(
                "⚠️ Skipping text vector for {} (t={}): dim mismatch ({} != {}).",
                path,
                row.timestamp_ms,
                row.dense.len(),
                BGE_DENSE_DIM
            );
            continue;
        }
        let blob = encode_f32_le(&row.dense);
        conn.execute(
            "INSERT INTO media_text_vectors
                (path, timestamp, model, dense_blob, sparse_json, chunk_index, parent_id)
             VALUES (?1, ?2, ?3, ?4, NULL, 0, NULL)",
            params![path, row.timestamp_ms, BGE_MODEL_TAG, blob],
        )
        .map_err(|e| format!("insert text vector failed: {}", e))?;
    }
    Ok(())
}

/// 插入一条纯文本条目（`text_entries` 表）。
///
/// 纯文本是**独立检索对象**（笔记 / 文档 / 外部转录），与媒体内嵌 OCR 文本
/// （`media_text_vectors`）是两张表、两条语义，绝不混存。
/// dense 维度不符直接报错，绝不静默写入错维度数据。
pub fn insert_text_entry(
    conn: &Connection,
    entry_id: &str,
    source_uri: &str,
    content: &str,
    dense: &[f32],
    metadata_json: Option<&str>,
) -> Result<(), String> {
    if dense.len() != BGE_DENSE_DIM {
        return Err(format!(
            "dense dim mismatch: expected {}, got {}",
            BGE_DENSE_DIM,
            dense.len()
        ));
    }
    let blob = encode_f32_le(dense);
    let index_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    conn.execute(
        "INSERT OR REPLACE INTO text_entries
            (entry_id, source_uri, content, model, dense_blob, sparse_json, index_time, metadata_json)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7)",
        params![
            entry_id,
            source_uri,
            content,
            BGE_MODEL_TAG,
            blob,
            index_time,
            metadata_json
        ],
    )
    .map_err(|e| format!("insert text entry failed: {}", e))?;
    Ok(())
}

/// 列出已入库的纯文本条目（管理视图：`先预览、后行动`的预览侧）。
/// 按入库时间倒序，最新的在前。
///
/// `query` 为子串关键词，按 `content` 精确包含过滤（供用户在大量条目里
/// 定位到想删的那几条）。传空串则不过滤。
/// 用 `'%' || ? || '%'` 拼 LIKE 模式——空串时得到 `'%%'`，匹配全部，
/// 因此不必为「有/无关键词」写两条 SQL。
pub fn list_text_entries(
    conn: &Connection,
    page: usize,
    limit: usize,
    query: &str,
) -> Result<TextEntryListResponse, String> {
    let trimmed = query.trim();
    let limit_i64 = limit as i64;
    let offset = (page.saturating_sub(1) * limit) as i64;

    // P2-1 / P1-15：FTS5 trigram 容错检索（优先），回退到转义 LIKE。
    // 债单 C6：trigram 对 <3 字符查询不命中，短查询回退 LIKE，
    // 避免搜「AI」「图」「猫」这类短词从「LIKE 能出结果」退化成静默 0 结果。
    let use_fts = !trimmed.is_empty() && trimmed.chars().count() >= 3 && try_fts_search(conn)?;
    let (total, rows) = if use_fts {
        // FTS5 路径：MATCH 查询，trigram 容错
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM text_entries_fts WHERE text_entries_fts MATCH ?1",
                params![trimmed],
                |r| r.get(0),
            )
            .map_err(|e| format!("count text entries (FTS) failed: {}", e))?;
        let mut stmt = conn
            .prepare(
                "SELECT te.entry_id, te.source_uri, te.content, te.index_time, te.metadata_json
                 FROM text_entries te
                 JOIN text_entries_fts fts ON te.entry_id = fts.entry_id
                 WHERE text_entries_fts MATCH ?1
                 ORDER BY te.index_time DESC LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| format!("prepare FTS query failed: {}", e))?;
        let rows = stmt
            .query_map(params![trimmed, limit_i64, offset], |row| {
                Ok(TextEntryItem {
                    entry_id: row.get(0)?,
                    source_uri: row.get(1).unwrap_or_default(),
                    content: row.get(2)?,
                    index_time: row.get(3).unwrap_or(0.0),
                    metadata_json: row.get(4).ok(),
                })
            })
            .map_err(|e| format!("FTS query failed: {}", e))?
            .filter_map(|r| r.ok())
            .collect();
        (total as usize, rows)
    } else {
        // LIKE 路径（FTS 不可用或空查询时）
        let escaped = trimmed.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("%{}%", escaped);
        let total: i64 = if trimmed.is_empty() {
            conn.query_row(
                "SELECT COUNT(*) FROM text_entries WHERE model = ?1",
                params![BGE_MODEL_TAG],
                |r| r.get(0),
            )
            .map_err(|e| format!("count text entries failed: {}", e))?
        } else {
            conn.query_row(
                "SELECT COUNT(*) FROM text_entries WHERE model = ?1 AND content LIKE ?2 ESCAPE '\\'",
                params![BGE_MODEL_TAG, pattern],
                |r| r.get(0),
            )
            .map_err(|e| format!("count text entries failed: {}", e))?
        };
        let mut stmt = if trimmed.is_empty() {
            conn.prepare(
                "SELECT entry_id, source_uri, content, index_time, metadata_json
                 FROM text_entries WHERE model = ?1
                 ORDER BY index_time DESC LIMIT ?2 OFFSET ?3",
            )
        } else {
            conn.prepare(
                "SELECT entry_id, source_uri, content, index_time, metadata_json
                 FROM text_entries WHERE model = ?1 AND content LIKE ?2 ESCAPE '\\'
                 ORDER BY index_time DESC LIMIT ?3 OFFSET ?4",
            )
        }.map_err(|e| format!("prepare list text entries failed: {}", e))?;

        let rows = if trimmed.is_empty() {
            stmt.query_map(params![BGE_MODEL_TAG, limit_i64, offset], map_text_entry_row)
                .map_err(|e| format!("query text entries failed: {}", e))?
                .filter_map(|r| r.ok())
                .collect()
        } else {
            stmt.query_map(params![BGE_MODEL_TAG, pattern, limit_i64, offset], map_text_entry_row)
                .map_err(|e| format!("query text entries failed: {}", e))?
                .filter_map(|r| r.ok())
                .collect()
        };
        (total as usize, rows)
    };

    Ok(TextEntryListResponse { items: rows, total_count: total })
}

/// 检测 FTS5 表是否存在（trigram tokenizer 需 SQLite 编译支持）。
fn try_fts_search(conn: &Connection) -> Result<bool, String> {
    match conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name='text_entries_fts' LIMIT 1",
        [],
        |_| Ok(true),
    ) {
        Ok(true) => Ok(true),
        _ => Ok(false),
    }
}

/// 把 rusqlite Row 映射为 TextEntryItem。
fn map_text_entry_row(row: &rusqlite::Row) -> rusqlite::Result<TextEntryItem> {
    Ok(TextEntryItem {
        entry_id: row.get(0)?,
        source_uri: row.get(1).unwrap_or_default(),
        content: row.get(2)?,
        index_time: row.get(3).unwrap_or(0.0),
        metadata_json: row.get(4).ok(),
    })
}

/// 删除纯文本条目（**终态，不可回退**），返回实际删除的行数。
///
/// 调用方（命令层）必须让用户先勾选再执行——删除是不可逆动作，
/// 与幽灵清理的 `purge` 同理，绝不提供「一键清空」。
pub fn delete_text_entries(conn: &Connection, entry_ids: &[String]) -> Result<usize, String> {
    let mut removed = 0;
    for id in entry_ids {
        removed += conn
            .execute("DELETE FROM text_entries WHERE entry_id = ?1", params![id])
            .map_err(|e| format!("delete text entry failed: {}", e))?;
    }
    Ok(removed)
}

/// P1-11 E8：按来源文件删除所有旧块——ingest 前先清旧。
/// 第二次 ingest 同一文件若分块数变少，旧的高 idx 块不会被 INSERT OR REPLACE 覆盖，
/// 会永久残留并仍被搜出来。此函数在插入新块前调用，保证一致性。
pub fn delete_text_entries_by_source(conn: &Connection, source_uri: &str) -> Result<usize, String> {
    let removed = conn
        .execute("DELETE FROM text_entries WHERE source_uri = ?1", params![source_uri])
        .map_err(|e| format!("delete text entries by source failed: {}", e))?;
    Ok(removed)
}
