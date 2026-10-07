// =========================================================================
//  Global Application Constants
// =========================================================================

/// 免费试用版最多允许索引的图片/视频帧数量
pub const FREE_TRIAL_LIMIT: usize = 100;

/// SigLIP 2.0 特征向量维度 (768 维)
pub const VECTOR_DIM: usize = 768;

/// BGE-M3 dense 文本向量维度 (1024 维)
/// 与 worker 侧 `config.py::BGE_DENSE_DIM` 保持一致。视觉（SigLIP 768）与文本
/// （BGE-M3 1024）是两条检索公路，维度不同、绝不混算。
pub const BGE_DENSE_DIM: usize = 1024;

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

/// 纯文本扩展名白名单（用于「文本文件直接入库」）。
/// 与图片/视频分开：这些文件的内容**不经过 SigLIP 视觉编码**，
/// 而是读入文本后走 BGE-M3 文本通道，落 `text_entries`。
///
/// **P1-11 / 第三轮 E12 修复**：旧实现只有 md/markdown/txt/text。
/// **债单 B6/C4 修复**：pdf/docx 是二进制格式，按 UTF-8 读会灌乱码进索引、
/// 污染检索结果且用户看不出原因（比「明确拒收」更糟）。在接入真正的文本提取库前，
/// 先撤回这两个扩展名，回到「明确拒收」——拖入时前端提示「暂不支持，请转成 txt/md」。
/// rtf 是文本标记格式（内含可读文本），仍按 UTF-8 读，保留。
pub const TEXT_EXTENSIONS: &[&str] = &[
    "md", "markdown", "txt", "text",
    "rtf", "log", "csv", "json",
    "srt", "vtt",  // 字幕
];

/// 纯文本分块的目标字符数。
/// 长文档必须切片——BGE-M3 有 token 上限（worker 侧 `BGE_MAX_TOKENS=512`），
/// 整篇塞进去会被截断。中文约 1 字 ≈ 1~1.5 token，故 500 字符是安全区间。
/// 分块信息记在 `metadata_json`（chunk_index / chunk_count），避免为此改表结构。
pub const TEXT_CHUNK_CHARS: usize = 500;

/// 聚类采样阈值（P2-7 / 第三轮 C9）：超过此帧数触发采样预筛，
/// 避免全量 O(N²) 点积导致界面假死。
pub const CLUSTER_SAMPLE_THRESHOLD: usize = 5000;

/// 文本分块的重叠字符数（P1-11 / 第三轮 E9）。
/// 跨块边界的那句关键话被切成两半时，两块的向量都不足以命中。
/// 10~15% overlap（500 × 0.1 = 50）让边界语句在两块中各保留一份，
/// 显著提升跨块召回。0 表示关闭（旧行为）。
pub const TEXT_CHUNK_OVERLAP: usize = 50;

// =========================================================================
//  分数架构（方案 C / 「后端不做决定」）
// =========================================================================
//  后端不再做 sigmoid 归一与加权求和。视觉通道返回**原始 SigLIP 余弦**
//  （`SearchResult.visual_similarity`），字面通道只返回命中布尔（`ocr_hit` /
//  `note_hit` / `filename_hit`）。分数如何融合、如何映射成人类可读分，
//  完全由前端（`utils/score.ts`）自决。故此处不再需要任何融合权重常量。

// =========================================================================
//  RPC 超时（毫秒）（P1-14 / 第三轮 C7）
// =========================================================================
//  旧实现全部用 -1（无限等待），worker 卡住 ⇒ UI 永久转圈只能杀进程。
//  分类超时：搜索类短超时快速失败，索引类按帧数估算留足时间。

/// 搜索/编码查询类 RPC 超时（毫秒）。SigLIP/BGE-M3 单次编码通常 <2s，5s 留足余量。
pub const RPC_TIMEOUT_SEARCH_MS: i32 = 5000;
/// 单文件任务（以图搜图的种子图编码）超时。图片通常 <3s，10s 兜底大图。
pub const RPC_TIMEOUT_FILE_TASK_MS: i32 = 10000;
/// 引擎就绪探测超时（毫秒）。`ping_engine` 用于探测 worker 是否完成模型加载，
/// 首次启动加载 SigLIP/BGE 模型可能需要数十秒，故给足 90s 长超时。
pub const RPC_TIMEOUT_PING_MS: i32 = 90000;
/// 索引批处理超时（毫秒）。批量索引/重索引/OCR 按批（4 个文件）提交，
/// 单批长视频可能数十秒，60s 兜底；超时即快速失败，不再无限等待（债单 A13）。
pub const RPC_TIMEOUT_INDEX_BASE_MS: i32 = 60000;
/// 单帧索引估算耗时（毫秒），用于未来动态计算索引批超时（本轮按批用 BASE_MS 兜底）。
#[allow(dead_code)]
pub const RPC_TIMEOUT_INDEX_PER_FRAME_MS: i32 = 250;