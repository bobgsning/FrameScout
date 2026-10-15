// search_unified — 统一搜索（P2-2 / 第三轮 A20「三条检索公路完全割裂」）。
//
// 旧实现：search_images / search_text / search_text_entries 三个独立命令，
// 前端拿到三份互不相干的列表，用户不知道该看哪一栏。
// 新实现：单遍扫描内存矩阵，返回统一列表 + 结构化命中来源 matches: [{channel, score}]，
// 供前端画分数来源条。
//
// 方案 C（「后端不做决定」）：不再 sigmoid 归一、不再加权求和。视觉通道返回**原始
// SigLIP 余弦**（visual_similarity），字面通道只返回命中布尔（ocr_hit/note_hit/
// filename_hit）。分数如何融合、如何映射成人类可读分，完全由前端自决。
//
// 注意：BGE-M3 文本语义通道（search_text）暂不纳入（需要独立的 ZMQ 编码调用），
// 留作后续扩展。

use std::collections::HashMap;
use tauri::State;
use crate::model_code::{AppState, OcrLineHit};
use crate::services::request_vector;
use crate::proto::framescout as proto;
use crate::constants::RPC_TIMEOUT_SEARCH_MS;
use crate::commands::search_cmd::{
    normalize_query, match_text, fetch_ocr_hit_lines, matches_file_types,
};
use crate::storage::path_key;

/// 统一搜索：单遍扫描矩阵，融合三通道，返回统一列表 + 结构化命中来源。
///
/// M2''' 分区搜索（P1-4）：新增可选过滤参数——来源批次 / 根目录前缀 / 入库时间范围。
/// 后端只做「事实性过滤」，不改变打分口径（仍只回原始余弦 + 命中布尔）。
#[tauri::command]
pub async fn search_unified(
    state: State<'_, AppState>,
    text: String,
    page: usize,
    limit: usize,
    use_vector: bool,
    use_ocr: bool,
    use_note: bool,
    use_filename: bool,
    batch_id: Option<String>,
    root_path: Option<String>,
    index_time_from: Option<f64>,
    index_time_to: Option<f64>,
    file_types: Option<Vec<String>>,
) -> Result<UnifiedSearchResponse, String> {
    {
        let memory = state.memory_db.read().map_err(|e| e.to_string())?;
        if memory.is_empty() {
            return Err("The library is empty. Please scan a folder first.".to_string());
        }
    }

    // 1) 查询预处理（与 search_images 同口径：trim + 全角折叠 + 切词 AND + 排除词）
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(UnifiedSearchResponse { items: Vec::new(), total_count: 0 });
    }
    let query_tokens = normalize_query(&text);

    // 2) 视觉语义编码（可选）
    let mut text_vec = Vec::new();
    if use_vector {
        // 债单 A13：超时换 RPC_TIMEOUT_SEARCH_MS，语义编码卡住时快速失败
        let text_frames = request_vector(proto::encode_request::Payload::Text(text.clone()), RPC_TIMEOUT_SEARCH_MS)?;
        if !text_frames.is_empty() {
            text_vec = text_frames[0].vector.clone();
        }
    }

    // 3) 单遍扫描矩阵，对每个帧计算各通道分数
    let memory = state.memory_db.read().map_err(|e| e.to_string())?;

    let vector_scores: HashMap<usize, f32> = if use_vector && !text_vec.is_empty() {
        memory.search(&text_vec, memory.len()).into_iter().collect()
    } else {
        HashMap::new()
    };

    let mut results: Vec<UnifiedSearchResult> = Vec::new();

    // M2''' 分区搜索：按批次过滤时，先从 SQL 取该批次的所有路径，构建大小写折叠键集合。
    let batch_paths: Option<std::collections::HashSet<String>> = match &batch_id {
        Some(bid) if !bid.is_empty() => {
            let db = state.db_conn.lock().map_err(|e| e.to_string())?;
            let mut stmt = db
                .prepare("SELECT DISTINCT path FROM frame_vectors WHERE indexed_in_batch_id = ?1")
                .map_err(|e| e.to_string())?;
            let paths: Vec<String> = stmt
                .query_map(rusqlite::params![bid], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            drop(stmt);
            let set: std::collections::HashSet<String> = paths.iter().map(|p| path_key(p)).collect();
            Some(set)
        }
        _ => None,
    };

    // M2''' 分区搜索：根目录前缀过滤（大小写不敏感、`\`/`/` 规范化、目录边界）。
    // 修复（第七轮）：旧实现用裸 `starts_with` 且未规范化分隔符，
    //   `C:\Users\a` 会误匹配 `C:\Users\abc`；`\` 与 `/` 混用也会漏匹配。
    let root_prefix: Option<String> = root_path.as_ref().map(|root| {
        let trimmed = root.trim();
        if trimmed.is_empty() {
            // 第七轮核查修复：空串会被 push 成 `"\`，把全部结果过滤掉。
            // 空目录条件 = 不限，返回空串并由下方 `!prefix.is_empty()` 跳过。
            return String::new();
        }
        let mut r = path_key(trimmed); // 规范化 + 小写 + 折叠分隔符
        if !r.ends_with('\\') {
            r.push('\\'); // 目录边界：确保按目录而非文件名前缀匹配
        }
        r
    });

    for (idx, meta) in memory.metadata.iter().enumerate() {
        if let Some(prefix) = &root_prefix {
            if !prefix.is_empty() && !path_key(&meta.path).starts_with(prefix) {
                continue;
            }
        }
        // M2''' 分区搜索：入库时间范围过滤。
        if let Some(from) = index_time_from {
            if meta.index_time < from {
                continue;
            }
        }
        if let Some(to) = index_time_to {
            if meta.index_time > to {
                continue;
            }
        }
        // M2''' 分区搜索：来源批次过滤。
        if let Some(set) = &batch_paths {
            if !set.contains(&path_key(&meta.path)) {
                continue;
            }
        }
        // 第七轮：file_type 条件（图片 / 视频）。
        if !matches_file_types(&meta.path, file_types.as_deref().unwrap_or(&[])) {
            continue;
        }

        let mut matches: Vec<ChannelMatch> = Vec::new();
        let mut ocr_hit = false;
        let mut note_hit = false;
        let mut filename_hit = false;

        // 视觉通道（方案 C「后端不做决定」）：返回原始 SigLIP 余弦，不 sigmoid、不加权。
        let raw_cosine = *vector_scores.get(&idx).unwrap_or(&0.0);
        let visual_similarity = if use_vector { raw_cosine } else { 0.0 };
        if visual_similarity > 0.001 {
            matches.push(ChannelMatch {
                channel: "visual".to_string(),
                score: visual_similarity,
            });
        }

        // OCR / 笔记 / 文件名：走 normalize_query + match_text（切词 AND，无符号语法）。
        if use_ocr && !meta.ocr_text.is_empty() && match_text(&meta.ocr_text.to_lowercase(), &query_tokens) {
            ocr_hit = true;
            matches.push(ChannelMatch { channel: "ocr".to_string(), score: 1.0 });
        }
        if use_note && !meta.user_note.is_empty() && match_text(&meta.user_note.to_lowercase(), &query_tokens) {
            note_hit = true;
            matches.push(ChannelMatch { channel: "note".to_string(), score: 1.0 });
        }
        if use_filename && match_text(&meta.path.to_lowercase(), &query_tokens) {
            filename_hit = true;
            matches.push(ChannelMatch { channel: "filename".to_string(), score: 1.0 });
        }

        // score 排序键：视觉命中 = 原始余弦；纯字面命中 = 命中通道数(0~3)。
        let hit_count = (ocr_hit as u8 + note_hit as u8 + filename_hit as u8) as f32;
        let score = if visual_similarity > 0.001 {
            visual_similarity
        } else {
            hit_count
        };

        let is_hit = visual_similarity > 0.001 || ocr_hit || note_hit || filename_hit;
        if is_hit {
            // 债单 A8：补 ocr_lines 回传（旧实现恒空，红框数据到不了前端）
            let ocr_lines = if ocr_hit {
                let db = state.db_conn.lock().map_err(|e| e.to_string())?;
                fetch_ocr_hit_lines(&db, &meta.path, &query_tokens)
            } else {
                Vec::new()
            };
            let match_count = ocr_lines.len() as u32;

            results.push(UnifiedSearchResult {
                path: meta.path.clone(),
                timestamp: meta.timestamp,
                score,
                matched_tags: matches.iter().map(|m| match m.channel.as_str() {
                    "visual" => "🖼️ Visual",
                    "ocr" => "🔍 OCR",
                    "note" => "📝 Note",
                    "filename" => "📁 Filename",
                    _ => "❓",
                }).map(|s| s.to_string()).collect(),
                ocr_text: meta.ocr_text.clone(),
                user_note: meta.user_note.clone(),
                index_time: meta.index_time,
                ocr_lines,
                match_count,
                matches,
                visual_similarity,
                ocr_hit,
                note_hit,
                filename_hit,
            });
        }
    }

    // 4) 按融合分数降序排序 + tie-breaker（与 search_images 一致）
    results.sort_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
    });

    // 5) 帧级收敛到文件级（取每文件最高分帧）
    let mut seen = std::collections::HashSet::new();
    let mut unique: Vec<UnifiedSearchResult> = Vec::new();
    for r in results {
        let key = path_key(&r.path);
        if !seen.contains(&key) {
            seen.insert(key);
            unique.push(r);
        }
    }

    let total_count = unique.len();
    // M2''' 布尔搜索：`limit == 0` 表示「返回全部原始命中、不截断分页」，
    // 供前端做 OR/AND/NOT 的集合组合。后端仍只回原始分，不做任何组合判定。
    let items = if limit == 0 {
        unique
    } else {
        let start = (page.saturating_sub(1)) * limit;
        unique.into_iter().skip(start).take(limit).collect()
    };

    Ok(UnifiedSearchResponse { items, total_count })
}

#[derive(serde::Serialize)]
pub struct ChannelMatch {
    pub channel: String,
    pub score: f32,
}

#[derive(serde::Serialize)]
pub struct UnifiedSearchResult {
    pub path: String,
    pub timestamp: f32,
    pub score: f32,
    pub matched_tags: Vec<String>,
    pub ocr_text: String,
    pub user_note: String,
    pub index_time: f64,
    pub ocr_lines: Vec<OcrLineHit>,
    pub match_count: u32,
    pub matches: Vec<ChannelMatch>,
    /// 原始 SigLIP 视觉余弦（不 sigmoid）；非视觉通道恒为 0。
    #[serde(default)]
    pub visual_similarity: f32,
    #[serde(default)]
    pub ocr_hit: bool,
    #[serde(default)]
    pub note_hit: bool,
    #[serde(default)]
    pub filename_hit: bool,
}

#[derive(serde::Serialize)]
pub struct UnifiedSearchResponse {
    pub items: Vec<UnifiedSearchResult>,
    pub total_count: usize,
}
