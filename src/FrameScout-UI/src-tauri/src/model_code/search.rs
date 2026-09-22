use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct SearchResult {
    pub path: String,
    pub timestamp: f32,
    pub score: f32,
    pub matched_tags: Vec<String>,
    pub ocr_text: String,
    pub user_note: String,
    pub index_time: f64,
}

#[derive(Serialize)]
pub struct PagedResponse {
    pub items: Vec<SearchResult>,
    pub total_count: usize,
}