// 负责文本搜图（多模态混合检索）、以图搜图以及心跳 Ping。

use tauri::State;
use crate::models::{AppState, PagedResponse, SearchResult};
use crate::proto::framescout as proto;
use crate::services::request_vector;

#[tauri::command]
pub async fn ping_engine() -> Result<String, String> {
    match request_vector(proto::encode_request::Payload::Text("PING_ENGINE".to_string()), 90000) {
        Ok(_) => Ok("READY".to_string()),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn search_images(
    state: State<'_, AppState>,
    text: String,
    page: usize,
    limit: usize,
    use_vector: bool,
    use_ocr: bool,
    use_note: bool,
    use_filename: bool,
) -> Result<PagedResponse, String> {
    {
        let memory = state.memory_db.read().map_err(|e| e.to_string())?;
        if memory.is_empty() {
            return Err("Memory Matrix is empty! Please scan a folder first.".to_string());
        }
    }

    let mut text_vec = Vec::new();
    if use_vector {
        let text_frames = request_vector(proto::encode_request::Payload::Text(text.clone()), -1)?;
        if !text_frames.is_empty() {
            text_vec = text_frames[0].2.clone();
        }
    }

    let memory = state.memory_db.read().map_err(|e| e.to_string())?;
    if memory.is_empty() {
        return Err("Memory Matrix is empty!".to_string());
    }

    let vector_scores = if use_vector && !text_vec.is_empty() {
        memory.search(&text_vec, memory.len())
    } else {
        Vec::new()
    };

    let score_map: std::collections::HashMap<usize, f32> = vector_scores.into_iter().collect();
    let lower_search = text.to_lowercase();
    let mut results: Vec<SearchResult> = Vec::new();

    for (idx, meta) in memory.metadata.iter().enumerate() {
        let mut score: f32 = *score_map.get(&idx).unwrap_or(&0.0);
        let mut matched_tags = Vec::new();

        if score > 0.001 {
            matched_tags.push("💡 Semantic".to_string());
        } else {
            score = 0.0;
        }

        if use_ocr && !meta.ocr_text.is_empty() && meta.ocr_text.to_lowercase().contains(&lower_search) {
            score += 2.0;
            matched_tags.push("🔍 OCR".to_string());
        }
        if use_note && !meta.user_note.is_empty() && meta.user_note.to_lowercase().contains(&lower_search) {
            score += 2.5;
            matched_tags.push("📝 Note".to_string());
        }
        if use_filename && meta.path.to_lowercase().contains(&lower_search) {
            score += 3.0;
            matched_tags.push("📁 Filename".to_string());
        }

        if score > 0.001 || !matched_tags.is_empty() {
            results.push(SearchResult {
                path: meta.path.clone(),
                timestamp: meta.timestamp,
                score,
                matched_tags,
                ocr_text: meta.ocr_text.clone(),
                user_note: meta.user_note.clone(),
                index_time: meta.index_time,
            });
        }
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // 帧级命中收敛到文件级：results 已按 score 降序排列，故同一路径
    // 首次命中即为该文件得分最高的那一帧；保留其 timestamp 供前端秒级跳转。
    // （视频多帧、图片单帧均适用；图片 timestamp 恒为 0.0。）
    let mut unique_results = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();
    for res in results {
        let normalized = res.path.to_lowercase().replace("\\", "/");
        if !seen_paths.contains(&normalized) {
            seen_paths.insert(normalized);
            unique_results.push(res);
        }
    }

    let total_count = unique_results.len();
    let start_index = (page.saturating_sub(1)) * limit;
    let items = unique_results.into_iter().skip(start_index).take(limit).collect();
    Ok(PagedResponse { items, total_count })
}

#[tauri::command]
pub async fn search_by_image(
    state: State<'_, AppState>,
    image_path: String,
    page: usize,
    limit: usize,
) -> Result<PagedResponse, String> {
    {
        let memory = state.memory_db.read().map_err(|e| e.to_string())?;
        if memory.is_empty() {
            return Err("Memory Matrix is empty! Please scan a folder first.".to_string());
        }
    }

    let img_frames = request_vector(proto::encode_request::Payload::FilePath(image_path), -1)?;
    if img_frames.is_empty() {
        return Err("Failed to extract image vectors".to_string());
    }
    let search_vec = &img_frames[0].2;

    let memory = state.memory_db.read().map_err(|e| e.to_string())?;
    if memory.is_empty() {
        return Err("Memory Matrix is empty!".to_string());
    }

    let vector_scores = memory.search(search_vec, memory.len());

    let mut results: Vec<SearchResult> = vector_scores
        .into_iter()
        .filter_map(|(idx, score)| {
            if score > 0.001 {
                let meta = &memory.metadata[idx];
                Some(SearchResult {
                    path: meta.path.clone(),
                    timestamp: meta.timestamp,
                    score,
                    matched_tags: vec!["🖼️ Visual".to_string()],
                    ocr_text: meta.ocr_text.clone(),
                    user_note: meta.user_note.clone(),
                    index_time: meta.index_time,
                })
            } else {
                None
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // 帧级命中收敛到文件级：results 已按 score 降序排列，故同一路径
    // 首次命中即为该文件得分最高的那一帧；保留其 timestamp 供前端秒级跳转。
    // （视频多帧、图片单帧均适用；图片 timestamp 恒为 0.0。）
    let mut unique_results = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();
    for res in results {
        let normalized = res.path.to_lowercase().replace("\\", "/");
        if !seen_paths.contains(&normalized) {
            seen_paths.insert(normalized);
            unique_results.push(res);
        }
    }

    let total_count = unique_results.len();
    let start_index = (page.saturating_sub(1)) * limit;
    let items = unique_results.into_iter().skip(start_index).take(limit).collect();
    Ok(PagedResponse { items, total_count })
}