use serde::Serialize;

/// 一条 OCR 命中行（P1-2 / 第三轮 A22）。
/// 唤醒沉睡的 `media_ocr_entries` 表——它存了 bbox/conf/lang 但全仓无 SELECT。
/// 检索命中时把匹配的行级 OCR 带回前端，供画红框高亮与「N 处匹配」徽标。
#[derive(Serialize, Clone)]
pub struct OcrLineHit {
    /// 命中文字
    pub text: String,
    /// 归一化 bbox（0~1）
    pub bbox_left: f32,
    pub bbox_top: f32,
    pub bbox_right: f32,
    pub bbox_bottom: f32,
    /// 置信度（0~1）
    pub conf: f32,
    /// 语言标签
    pub lang: String,
    /// 帧时间戳（秒）
    pub timestamp: f32,
}

#[derive(Serialize, Clone)]
pub struct SearchResult {
    pub path: String,
    pub timestamp: f32,
    /// 排序键（方案 C）：视觉命中 = 原始 SigLIP 余弦；纯字面命中 = 命中通道数(0~3)。
    /// 前端**不**用此字段算展示分数，改用 `visual_similarity` / `*_hit` 自决映射。
    pub score: f32,
    pub matched_tags: Vec<String>,
    pub ocr_text: String,
    pub user_note: String,
    pub index_time: f64,
    /// 命中的 OCR 行级数据（P1-2）：含 bbox 供前端画红框。
    /// 仅在 OCR 通道命中且 media_ocr_entries 有数据时填充。
    #[serde(default)]
    pub ocr_lines: Vec<OcrLineHit>,
    /// 命中处数（P1-2 / 第三轮「N 处匹配」徽标）。
    /// 视频多帧命中时此处聚合所有帧的命中行数。
    #[serde(default)]
    pub match_count: u32,
    /// 原始 SigLIP 视觉余弦（方案 C / 「后端不做决定」）：**不 sigmoid、不加权**，
    /// 0~1。非视觉通道（exact / 浏览）恒为 0。前端据此用 mapToHumanScore 自决映射。
    #[serde(default)]
    pub visual_similarity: f32,
    /// 各字面通道命中详情（方案 C）：供前端画得分来源条、自决融合。
    #[serde(default)]
    pub ocr_hit: bool,
    #[serde(default)]
    pub note_hit: bool,
    #[serde(default)]
    pub filename_hit: bool,
}

/// BGE-M3 文本语义检索的一条命中（「用文字搜媒体」通道）。
///
/// 这是**第一手数据**：`similarity` 是 query 与该帧 OCR 文本的 BGE-M3 dense 余弦
/// （原始、未加权），不做任何跨通道聚合——是否与视觉 / 精确匹配混合、怎么加权，
/// 由前端按用户选择映射，Rust 不替用户做决定（对应「提供信息，不给行为」）。
#[derive(Serialize, Clone)]
pub struct TextSearchHit {
    pub path: String,
    /// 视频帧时间戳（秒）；图片恒为 0.0。与 SearchResult.timestamp 对齐。
    pub timestamp: f32,
    /// BGE-M3 dense 余弦相似度（原始分数，未加工）
    pub similarity: f32,
    /// 该帧 OCR 文本（命中来源，供前端展示「为什么命中」）
    pub ocr_text: String,
    pub user_note: String,
    pub index_time: f64,
}

#[derive(Serialize)]
pub struct PagedResponse {
    pub items: Vec<SearchResult>,
    pub total_count: usize,
}

/// BGE-M3 文本检索的分页响应（items 为 TextSearchHit）。
#[derive(Serialize)]
pub struct TextPagedResponse {
    pub items: Vec<TextSearchHit>,
    pub total_count: usize,
}

/// 纯文本条目（`text_entries`）检索的一条命中。
/// `similarity` 是 query 与该条目内容的 BGE-M3 dense 余弦（第一手原始分）。
#[derive(Serialize, Clone)]
pub struct TextEntryHit {
    pub entry_id: String,
    pub source_uri: String,
    pub content: String,
    pub similarity: f32,
    pub index_time: f64,
}

/// 纯文本条目的一条记录（管理视图用：列出库里已有哪些条目）。
/// `metadata_json` 承载分块信息（chunk_index / chunk_count）等扩展数据。
#[derive(Serialize, Clone)]
pub struct TextEntryItem {
    pub entry_id: String,
    pub source_uri: String,
    pub content: String,
    pub index_time: f64,
    pub metadata_json: Option<String>,
}

/// 纯文本条目列表的分页响应（管理视图）。
#[derive(Serialize)]
pub struct TextEntryListResponse {
    pub items: Vec<TextEntryItem>,
    pub total_count: usize,
}

/// 纯文本条目检索的分页响应。
#[derive(Serialize)]
pub struct TextEntryPagedResponse {
    pub items: Vec<TextEntryHit>,
    pub total_count: usize,
}