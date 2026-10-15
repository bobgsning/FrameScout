// 纯文本通道的数据模型（入库结果 / 管理动作）。
//
// 纯文本是**独立检索对象**（笔记 / 文档 / 外部转录），与媒体内嵌 OCR 文本
// （`media_text_vectors`）分表分查、语义分离。

use serde::Serialize;

/// 一个文本文件入库失败的明细。
#[derive(Serialize, Clone)]
pub struct TextIngestFailure {
    pub path: String,
    pub error: String,
}

/// 文本文件批量入库的结果。
///
/// 一个长文档会被切成多块（每块一条 `text_entries` 行），故
/// `chunks` 通常大于 `ingested_files.len()`。
#[derive(Serialize)]
pub struct TextIngestResult {
    /// 成功入库的文件路径
    pub ingested_files: Vec<String>,
    /// 实际写入的文本块数
    pub chunks: usize,
    /// 入库失败的文件（读不了 / 编码失败 / 扩展名不支持），供用户重试
    pub failed: Vec<TextIngestFailure>,
    /// M0''' 告知：重新导入时被替换掉的旧块数（删旧块前告知用户删了多少）。
    pub deleted_old_chunks: usize,
}

/// 入库前预演（删前预告的准确数字，非全库总数）。
///
/// 只读、不读文件、不编码、不落库——查「这次导入的目标里，
/// 有多少来源文件已在库里有旧块、一共多少块会被替换」。
#[derive(Serialize)]
pub struct TextIngestPreview {
    /// 将被替换掉的旧块总数（目标文件里已在库中存在的 source_uri 的块数之和）
    pub will_replace_chunks: usize,
    /// 会动到的来源文件数（有多少个目标文件已有同来源旧块）
    pub affected_files: usize,
}
