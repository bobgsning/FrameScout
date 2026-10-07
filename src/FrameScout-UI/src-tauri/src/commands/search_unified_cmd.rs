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
use crate::commands::search_cmd::{normalize_query, match_text, fetch_ocr_hit_lines};
use crate::storage::path_key;

/// 统一搜索：单遍扫描矩阵，融合三通道，返回统一列表 + 结构化命中来源。
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

    for (idx, meta) in memory.metadata.iter().enumerate() {
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

        // OCR / 笔记 / 文件名：走 normalize_query + match_text（切词 AND + 排除词）。
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
    let start = (page.saturating_sub(1)) * limit;
    let items = unique.into_iter().skip(start).take(limit).collect();

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
