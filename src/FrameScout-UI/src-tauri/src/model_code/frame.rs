// 编码结果的内部数据模型（跨命令 / 跨层共享）。
//
// 历史上 `request_vector` 返回 5 元组 `(path, timestamp, vector, ocr_text, index_time)`；
// 协议加入行级 OCR 后元组已表达不动，故收拢为结构化类型，调用点读字段名而非
// `.0 .2 .3`，可读性随协议增长不再退化。

use serde::Serialize;

/// 一条 OCR 行级结果（对应 proto 的 OcrLine）。
#[derive(Debug, Clone, Default, Serialize)]
pub struct OcrLine {
    pub text: String,
    /// 归一化 0~1 的直角包围盒 (left, top, right, bottom)
    pub bbox: (f32, f32, f32, f32),
    pub score: f32,
    pub lang: String,
    /// 视频帧时间戳（毫秒）；图片恒为 0
    pub timestamp_ms: i64,
    /// 原始四角坐标等扩展信息（JSON）
    pub payload_json: String,
}

/// 一条编码完成的帧（图片单帧 / 视频一帧）。
#[derive(Debug, Clone, Default)]
pub struct EncodedFrame {
    pub path: String,
    pub timestamp: f32,
    /// SigLIP 2 视觉向量（768 维，用于以图搜图）
    pub vector: Vec<f32>,
    pub ocr_text: String,
    pub index_time: f64,
    pub ocr_lines: Vec<OcrLine>,
    /// BGE-M3 文本向量（1024 维 dense，非空即表示该帧 OCR 文字已抽成文本向量）。
    /// 与 `vector`（视觉）是两条检索公路，落库到 `media_text_vectors`（model='bge-m3'）。
    pub dense_vector: Vec<f32>,
}
