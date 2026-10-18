// 负责文本搜图（多模态混合检索）、以图搜图、BGE-M3 文本检索以及心跳 Ping。

use std::time::Instant;

use rusqlite::params;
use tauri::State;
use crate::constants::{BGE_DENSE_DIM, RPC_TIMEOUT_SEARCH_MS, RPC_TIMEOUT_FILE_TASK_MS, RPC_TIMEOUT_PING_MS};
use crate::error::{
    ec, ec1, ERR_BGE_UNAVAILABLE, ERR_DB_PREPARE_FAILED, ERR_DB_QUERY_FAILED, ERR_IMAGE_VECTOR_FAILED,
    ERR_INDEX_EMPTY, ERR_LOCK_POISONED, STATUS_ENGINE_READY, TAG_VISUAL,
};
use crate::fs_trace;
use crate::model_code::{
    AppState, OcrLineHit, PagedResponse, SearchResult, TextEntryHit, TextEntryPagedResponse,
    TextPagedResponse, TextSearchHit,
};
use crate::proto::framescout as proto;
use crate::services::{request_text_dense, request_text_entry, request_vector};
use crate::storage::text_store::{insert_text_entry as store_text_entry, BGE_MODEL_TAG};
use crate::storage::vector_blob::decode_f32_le_checked;
// 第七轮核查修复：本文件原先有一份私有 `path_key`（`\`→`/`），
// 与 `storage::path_key`（normalize_path，统一为 `\`）形态相反，
// 导致 root_prefix 拼接出 `c:/users/a\` 这种永不匹配的前缀 ——
// 「以图搜图 + 目录条件」恒 0 结果。现统一使用 storage 版本，与 search_unified_cmd 同口径。
use crate::storage::path_key;

// =========================================================================
//  查询预处理与融合打分（P0-3 / P0-4 / 第三轮 A1-A7）
// =========================================================================
//  旧实现：`text.to_lowercase()` 直接做 `contains` 子串匹配，三大痛：
//   1. 无 trim：复制粘贴带尾随空格 ⇒ 0 结果；
//   2. 整串当一词：搜「cat dog」要求文本里真有连续串「cat dog」；
//   3. 无 NFKC：全角「白板」与半角不同（虽然 CJK 不受影响，但拉丁与标点会）。
//  新实现：trim + 全角折叠 + lowercase + 切词 + AND 语义 + 排除词（-前缀）。

/// 全角→半角折叠（NFKC 子集，覆盖最常见的全角 ASCII 与全角空格）。
/// 不引入 unicode-normalization 重依赖，对路径/OCR 文本里混入的全角字符足够。
fn fold_fullwidth(s: &str) -> String {
    s.chars().map(|c| {
        let u = c as u32;
        if u >= 0xFF01 && u <= 0xFF5E {
            // 全角 ASCII !..~ → 半角 !..~
            char::from_u32(u - 0xFEE0).unwrap_or(c)
        } else if u == 0x3000 {
            // 全角空格 → 普通空格
            ' '
        } else {
            c
        }
    }).collect()
}

/// 切词后的查询：正向词必须全部命中（AND 语义）。
/// 第七轮核查修复：原先这里还有 `negative`（`-xxx` 排除词）语义，
/// 前端逻辑搜索重构已经把 `-` 从「语法」降级为「普通字符」，
/// 但后端仍在悄悄执行排除 —— 用户敲 `e-mail` / `-5°C` 会被改写。
/// 现彻底移除负向语义：**用户输入即意图**，`-` 只是普通字符。
#[derive(Default, Clone)]
pub(crate) struct QueryTokens {
    pub(crate) positive: Vec<String>,
}

/// 查询预处理：trim → 全角折叠 → 小写 → 切词（多词 AND）。
pub(crate) fn normalize_query(text: &str) -> QueryTokens {
    let folded = fold_fullwidth(text.trim());
    let lower = folded.to_lowercase();
    let mut tokens = QueryTokens::default();
    for tok in lower.split_whitespace() {
        if tok.is_empty() {
            continue;
        }
        tokens.positive.push(tok.to_string());
    }
    tokens
}

/// 在已小写的 haystack 上做切词 AND 匹配。
/// - 全部 positive token 必须 substring 命中；
/// - positive 为空时视为「不参与字面量匹配」（返回 false）。
///   债单 B11/C1 修复：空查询会让所有文件都「命中」并计分 ⇒ 召回全库，故直接返回 false。
pub(crate) fn match_text(haystack_lower: &str, tokens: &QueryTokens) -> bool {
    if tokens.positive.is_empty() {
        // 无正向词（空查询）：字面量通道不参与打分。
        return false;
    }
    for tok in &tokens.positive {
        if !haystack_lower.contains(tok) {
            return false;
        }
    }
    true
}

/// 路径归一化键：统一走 `crate::storage::path_key`（`normalize_path` + 小写）。
/// 第七轮核查修复：原先本文件自带一份 `to_lowercase().replace("\\", "/")` 的私有实现，
/// 与 storage 版（分隔符为 `\`）形态相反，已删除 —— 见文件头的 use 说明。

/// 文件类型归类（第七轮 `file_type` 条件用）：库里的媒体条目非视频即图片。
/// 与前端 `utils/media.ts::isVideo` 的扩展名集合保持一致。
pub(crate) fn file_kind(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "wmv" | "flv" | "m4v" | "mpg" | "mpeg" | "ts"
        | "3gp" | "ogv" => "video",
        _ => "image",
    }
}

/// `file_types` 过滤（空 / None = 不限）。
pub(crate) fn matches_file_types(path: &str, types: &[String]) -> bool {
    if types.is_empty() {
        return true;
    }
    let kind = file_kind(path);
    types.iter().any(|t| {
        let t = t.trim().to_lowercase();
        t == kind || (t == "images" && kind == "image") || (t == "videos" && kind == "video")
    })
}

/// 从 `media_ocr_entries` 查询命中文件的相关 OCR 行（P1-2 / 第三轮 A22）。
///
/// 唤醒沉睡的行级 OCR 表——它存了 bbox/conf/lang 但全仓无 SELECT。
/// 给定文件路径与查询词，返回该文件中包含查询词的 OCR 行（含 bbox），
/// 供前端画红框高亮与「N 处匹配」徽标。
///
/// 只返回文本包含任一 positive token 的行（与 match_text 同一语义）。
pub(crate) fn fetch_ocr_hit_lines(
    conn: &rusqlite::Connection,
    path: &str,
    tokens: &QueryTokens,
) -> Vec<OcrLineHit> {
    if tokens.positive.is_empty() {
        return Vec::new();
    }
    let mut stmt = match conn.prepare(
        "SELECT text_chunk, bbox_left, bbox_top, bbox_right, bbox_bottom, conf, timestamp
         FROM media_ocr_entries WHERE path = ?1 ORDER BY timestamp ASC, conf DESC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map(params![path], |row| {
        Ok((
            row.get::<_, String>(0).unwrap_or_default(),
            row.get::<_, f32>(1).unwrap_or(0.0),
            row.get::<_, f32>(2).unwrap_or(0.0),
            row.get::<_, f32>(3).unwrap_or(0.0),
            row.get::<_, f32>(4).unwrap_or(0.0),
            row.get::<_, f32>(5).unwrap_or(0.0),
            row.get::<_, i64>(6).unwrap_or(0),
        ))
    });
    let mut hits = Vec::new();
    if let Ok(rows) = rows {
        for row in rows.flatten() {
            let (text, bl, bt, br, bb, conf, ts_ms) = row;
            let text_lower = text.to_lowercase();
            // 命中判定：positive 全部命中（与 match_text 同口径）
            let mut ok = true;
            for tok in &tokens.positive {
                if !text_lower.contains(tok) {
                    ok = false;
                    break;
                }
            }
            if ok {
                hits.push(OcrLineHit {
                    text,
                    bbox_left: bl,
                    bbox_top: bt,
                    bbox_right: br,
                    bbox_bottom: bb,
                    conf,
                    lang: String::new(),
                    timestamp: ts_ms as f32 / 1000.0,
                });
            }
        }
    }
    hits
}

#[tauri::command]
pub async fn ping_engine() -> Result<String, String> {
    // 债单 A13：硬编码 90000 换常量 RPC_TIMEOUT_PING_MS（长超时等待模型加载）
    match request_vector(proto::encode_request::Payload::Text("PING_ENGINE".to_string()), RPC_TIMEOUT_PING_MS) {
        Ok(_) => Ok(ec(STATUS_ENGINE_READY)),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn search_by_image(
    state: State<'_, AppState>,
    image_path: String,
    page: usize,
    limit: usize,
    batch_id: Option<String>,
    root_path: Option<String>,
    index_time_from: Option<f64>,
    index_time_to: Option<f64>,
    file_types: Option<Vec<String>>,
) -> Result<PagedResponse, String> {
    {
        let memory = state.memory_db.read().map_err(|e| ec1(ERR_LOCK_POISONED, e))?;
        if memory.is_empty() {
            return Err(ec(ERR_INDEX_EMPTY));
        }
    }

    let search_started = Instant::now();

    // 以图搜图排除查询图自身需要 path_key（第三轮 A28），提前算好避免 borrow-after-move。
    let self_key = path_key(&image_path);

    // 以图搜图：只取视觉向量，不需要 OCR，故不配置 ocr_config。
    let img_frames = request_vector(
        proto::encode_request::Payload::FileTask(proto::SingleFileTask {
            file_path: image_path,
            ocr_config: None,
        }),
        RPC_TIMEOUT_FILE_TASK_MS,
    )?;
    if img_frames.is_empty() {
        return Err(ec(ERR_IMAGE_VECTOR_FAILED));
    }
    let search_vec = &img_frames[0].vector;

    let memory = state.memory_db.read().map_err(|e| ec1(ERR_LOCK_POISONED, e))?;
    if memory.is_empty() {
        return Err(ec(ERR_INDEX_EMPTY));
    }

    // M2''' 分区搜索（第七轮「全接」）：以图搜图也支持来源批次 / 根目录 / 时间过滤，
    // 使「基准图」条件能与时间/目录/批次条件组合，不再静默失效。
    let batch_paths: Option<std::collections::HashSet<String>> = match &batch_id {
        Some(bid) if !bid.is_empty() => {
            let db = state.db_conn.lock().map_err(|e| ec1(ERR_LOCK_POISONED, e))?;
            let mut stmt = db
                .prepare("SELECT DISTINCT path FROM frame_vectors WHERE indexed_in_batch_id = ?1")
                .map_err(|e| ec1(ERR_DB_PREPARE_FAILED, e))?;
            let paths: Vec<String> = stmt
                .query_map(rusqlite::params![bid], |row| row.get::<_, String>(0))
                .map_err(|e| ec1(ERR_DB_QUERY_FAILED, e))?
                .filter_map(|r| r.ok())
                .collect();
            drop(stmt);
            drop(db);
            let set: std::collections::HashSet<String> = paths.iter().map(|p| path_key(p)).collect();
            Some(set)
        }
        _ => None,
    };
    let root_prefix: Option<String> = root_path.as_ref().map(|root| {
        let trimmed = root.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        // 第七轮核查修复：原先这里按私有 path_key（分隔符为 `/`）拼了一个 `\`，
        // 产出 `c:/users/a\` 这种永不匹配的前缀 ⇒ 传了目录条件就恒 0 结果。
        // 现与 search_unified_cmd 同口径：storage::path_key（分隔符 `\`）+ 尾部补边界。
        let mut r = path_key(trimmed);
        if !r.ends_with('\\') {
            r.push('\\');
        }
        r
    });

    let vector_scores = memory.search(search_vec, memory.len());

    let mut results: Vec<SearchResult> = vector_scores
        .into_iter()
        .filter_map(|(idx, score)| {
            if score <= 0.001 {
                return None;
            }
            let meta = &memory.metadata[idx];
            // 排除种子图自身（按 path_key 比较，容忍大小写/分隔符差异）。
            if path_key(&meta.path) == self_key {
                return None;
            }
            // 分区过滤（与 search_unified 同口径）
            if let Some(prefix) = &root_prefix {
                if !prefix.is_empty() && !path_key(&meta.path).starts_with(prefix) {
                    return None;
                }
            }
            if let Some(from) = index_time_from {
                if meta.index_time < from {
                    return None;
                }
            }
            if let Some(to) = index_time_to {
                if meta.index_time > to {
                    return None;
                }
            }
            if let Some(set) = &batch_paths {
                if !set.contains(&path_key(&meta.path)) {
                    return None;
                }
            }
            // 第七轮：file_type 条件（图片 / 视频）
            if !matches_file_types(&meta.path, file_types.as_deref().unwrap_or(&[])) {
                return None;
            }
            Some(SearchResult {
                path: meta.path.clone(),
                timestamp: meta.timestamp,
                score,
                matched_tags: vec![ec(TAG_VISUAL)],
                ocr_text: meta.ocr_text.clone(),
                user_note: meta.user_note.clone(),
                index_time: meta.index_time,
                // 以图搜图不涉及文字匹配，无 OCR 命中行
                ocr_lines: Vec::new(),
                match_count: 0,
                // 以图搜图：原始视觉余弦即 score；无字面命中。
                visual_similarity: score,
                ocr_hit: false,
                note_hit: false,
                filename_hit: false,
            })
        })
        .collect();

    // 稳定排序 + tie-breaker（第三轮 A10）：同分时按 path 升序。
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
    });

    // 帧级命中收敛到文件级：results 已按 score 降序排列，故同一路径
    // 首次命中即为该文件得分最高的那一帧；保留其 timestamp 供前端秒级跳转。
    // （视频多帧、图片单帧均适用；图片 timestamp 恒为 0.0。）
    let mut unique_results = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();
    for res in results {
        let normalized = path_key(&res.path);
        if !seen_paths.contains(&normalized) {
            seen_paths.insert(normalized);
            unique_results.push(res);
        }
    }

    let total_count = unique_results.len();
    // 第八轮核查修复：`limit == 0` 与 search_unified 同约定 —— 表示「返回全部原始命中、
    // 不截断分页」，供前端做 OR/AND/NOT 的集合组合。此前这里直接 `take(0)` 截断为空，
    // 导致「以图搜图 + 第二个条件 / 排除 / 换排序」走 L3 拆叶子时基准图叶子恒空 ⇒ 整组 0 结果。
    let items = if limit == 0 {
        unique_results
    } else {
        let start_index = (page.saturating_sub(1)) * limit;
        unique_results.into_iter().skip(start_index).take(limit).collect()
    };

    fs_trace!(
        "search done, mode=image, page={}, limit={}, hits={}, took={:?}",
        page,
        limit,
        total_count,
        search_started.elapsed()
    );

    Ok(PagedResponse { items, total_count })
}

/// BGE-M3 文本语义检索：query 的 dense 与库里 `media_text_vectors` 的 dense 算余弦，
/// 返回**第一手原始数据**（每命中一个原始 `similarity`，不做任何跨通道加权）。
///
/// 与 `search_images`（视觉混合检索）是两条公路：本命令只走文本通道（OCR 文本向量），
/// 是否与视觉 / 精确匹配混合、怎么加权，由前端按用户选择映射——Rust 不替用户做决定。
#[tauri::command]
pub async fn search_text(
    state: State<'_, AppState>,
    text: String,
    page: usize,
    limit: usize,
) -> Result<TextPagedResponse, String> {
    let search_started = Instant::now();

    // 1. query → BGE-M3 dense（文本语义，查 OCR 文本向量）
    let query_dense = request_text_dense(text, RPC_TIMEOUT_SEARCH_MS)?;
    if query_dense.is_empty() {
        return Err(ec(ERR_BGE_UNAVAILABLE));
    }

    // 2. 从 SQLite 读 media_text_vectors 的 dense_blob，暴力余弦。
    //    media_text_vectors.timestamp 是毫秒、frame_vectors.timestamp 是秒，
    //    故 JOIN 时除以 1000 并留 0.001s 浮点容差（同源写入，误差远小于此）。
    //
    //    **必须 JOIN files 过滤 is_dead = 0（P0-2 / 第三轮 A18）**：旧实现只有
    //    `WHERE mtv.model = ?1`，完全没 JOIN files，于是「彻底删除」的文件
    //    在 purge 后仍能被文本搜出来——对以「数据主权」为品牌核心的项目是品牌级失信。
    //    内存矩阵加载时已过滤 is_dead（db.rs load_frame_rows），文本检索也必须对齐。
    let db = state.db_conn.lock().unwrap();
    let mut stmt = db
        .prepare(
            "SELECT mtv.path, mtv.timestamp, mtv.dense_blob, fv.ocr_text, fv.user_note, fv.index_time
             FROM media_text_vectors mtv
             LEFT JOIN frame_vectors fv
                 ON fv.path = mtv.path
                 AND ABS(fv.timestamp - (mtv.timestamp / 1000.0)) < 0.001
             LEFT JOIN files f ON f.path = mtv.path
             WHERE mtv.model = ?1
                 AND (f.is_dead IS NULL OR f.is_dead = 0)",
        )
        .map_err(|e| ec1(ERR_DB_PREPARE_FAILED, e))?;

    let mut hits: Vec<TextSearchHit> = Vec::new();
    let rows = stmt
        .query_map(params![BGE_MODEL_TAG], |row| {
            let path: String = row.get(0)?;
            let ts_ms: i64 = row.get(1)?;
            let blob: Option<Vec<u8>> = row.get(2)?;
            let ocr_text: String = row.get(3).unwrap_or_default();
            let user_note: String = row.get(4).unwrap_or_default();
            let index_time: f64 = row.get(5).unwrap_or(0.0);
            Ok((path, ts_ms, blob, ocr_text, user_note, index_time))
        })
        .map_err(|e| ec1(ERR_DB_QUERY_FAILED, e))?;

    for row in rows {
        let Ok((path, ts_ms, blob, ocr_text, user_note, index_time)) = row else {
            continue;
        };
        let dense = match blob.as_deref().and_then(|b| decode_f32_le_checked(b, BGE_DENSE_DIM)) {
            Some(v) => v,
            None => continue,
        };
        // BGE-M3 dense 已在 ONNX 导出时 L2 归一化，点积即余弦
        let similarity: f32 = query_dense.iter().zip(dense.iter()).map(|(a, b)| a * b).sum();
        hits.push(TextSearchHit {
            path,
            timestamp: ts_ms as f32 / 1000.0,
            similarity,
            ocr_text,
            user_note,
            index_time,
        });
    }
    drop(stmt);

    // 3. 按原始相似度降序，帧级命中收敛到文件级（与 search_images 同一规则）。
    //    加 tie-breaker（path 升序）避免同分时翻页重复/漏项（第三轮 A10）。
    hits.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
    });

    let mut unique = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for hit in hits {
        let normalized = path_key(&hit.path);
        if !seen.contains(&normalized) {
            seen.insert(normalized);
            unique.push(hit);
        }
    }

    let total_count = unique.len();
    let start = (page.saturating_sub(1)) * limit;
    let items = unique.into_iter().skip(start).take(limit).collect();

    fs_trace!(
        "text search done, page={}, limit={}, hits={}, took={:?}",
        page,
        limit,
        total_count,
        search_started.elapsed()
    );

    Ok(TextPagedResponse { items, total_count })
}

/// 纯文本条目入库：worker 用 BGE-M3 编码文本，落 `text_entries` 表（唯一写入口）。
/// 返回生成的 entry_id，供前端回显。
#[tauri::command]
pub async fn insert_text_entry(
    state: State<'_, AppState>,
    text: String,
    source_uri: String,
) -> Result<String, String> {
    // 1. worker 编码纯文本 → (entry_id, dense)
    let (entry_id, dense) = request_text_entry(text.clone(), source_uri.clone())?;
    if dense.is_empty() {
        return Err(ec(ERR_BGE_UNAVAILABLE));
    }
    // 2. 落 text_entries（唯一写入口）
    let db = state.db_conn.lock().unwrap();
    store_text_entry(&db, &entry_id, &source_uri, &text, &dense, None)?;
    Ok(entry_id)
}

/// 纯文本条目检索：query 的 dense 与 `text_entries` 的 dense 算余弦，
/// 返回第一手原始数据（similarity 为未加权余弦）。
#[tauri::command]
pub async fn search_text_entries(
    state: State<'_, AppState>,
    text: String,
    page: usize,
    limit: usize,
) -> Result<TextEntryPagedResponse, String> {
    let search_started = Instant::now();

    // 1. query → BGE-M3 dense
    let query_dense = request_text_dense(text, RPC_TIMEOUT_SEARCH_MS)?;
    if query_dense.is_empty() {
        return Err(ec(ERR_BGE_UNAVAILABLE));
    }

    // 2. 读 text_entries 的 dense_blob，暴力余弦
    let db = state.db_conn.lock().unwrap();
    let mut stmt = db
        .prepare(
            "SELECT entry_id, source_uri, content, dense_blob, index_time
             FROM text_entries WHERE model = ?1",
        )
        .map_err(|e| ec1(ERR_DB_PREPARE_FAILED, e))?;

    let mut hits: Vec<TextEntryHit> = Vec::new();
    let rows = stmt
        .query_map(params![BGE_MODEL_TAG], |row| {
            let entry_id: String = row.get(0)?;
            let source_uri: String = row.get(1).unwrap_or_default();
            let content: String = row.get(2)?;
            let blob: Option<Vec<u8>> = row.get(3)?;
            let index_time: f64 = row.get(4).unwrap_or(0.0);
            Ok((entry_id, source_uri, content, blob, index_time))
        })
        .map_err(|e| ec1(ERR_DB_QUERY_FAILED, e))?;

    for row in rows {
        let Ok((entry_id, source_uri, content, blob, index_time)) = row else {
            continue;
        };
        let dense = match blob.as_deref().and_then(|b| decode_f32_le_checked(b, BGE_DENSE_DIM)) {
            Some(v) => v,
            None => continue,
        };
        let similarity: f32 = query_dense.iter().zip(dense.iter()).map(|(a, b)| a * b).sum();
        hits.push(TextEntryHit {
            entry_id,
            source_uri,
            content,
            similarity,
            index_time,
        });
    }
    drop(stmt);

    // 3. 按原始相似度降序 + 分页。加 tie-breaker（entry_id 升序）避免同分时顺序不定。
    hits.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.entry_id.cmp(&b.entry_id))
    });

    let total_count = hits.len();
    let start = (page.saturating_sub(1)) * limit;
    let items = hits.into_iter().skip(start).take(limit).collect();

    fs_trace!(
        "text entries search done, page={}, limit={}, hits={}, took={:?}",
        page,
        limit,
        total_count,
        search_started.elapsed()
    );

    Ok(TextEntryPagedResponse { items, total_count })
}