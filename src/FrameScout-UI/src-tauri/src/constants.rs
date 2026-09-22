// =========================================================================
//  Global Application Constants
// =========================================================================

/// 免费试用版最多允许索引的图片/视频帧数量
pub const FREE_TRIAL_LIMIT: usize = 100;

/// SigLIP 2.0 特征向量维度 (768 维)
pub const VECTOR_DIM: usize = 768;

// =========================================================================
//  扩展名白名单（不带点，已与 extension() 结果归一化比较）
// =========================================================================
// 注意：此处两份清单必须与 Python 侧
// `src/inference-worker/media/video_extractor.py::VIDEO_EXTENSIONS`
// 保持一致，否则会出现"扫描能枚举到、却无法被 worker 正确处理"的静默丢帧。
// 修改任一侧时务必同步另一侧。

/// 图片扩展名白名单（用于扫描模式过滤）。
pub const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

/// 视频扩展名白名单（用于扫描模式过滤）。
/// 相比历史实现补齐了 .m4v / .ts，使其与 Python 侧一致。
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "avi", "mkv", "webm", "flv", "m4v", "ts"];