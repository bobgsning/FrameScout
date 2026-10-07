// 扫描报告 / 活动记录的数据模型。
//
// 每次批量操作完成时，摘要写进 `batches.summary_json` 持久化（应用重启不丢），
// UI 读 `WHERE batch_id = ?` 即可。这是「让用户有掌控感」的那份清单，
// 也是活动中心的雏形。

use serde::Serialize;

/// 一条批次记录（骨架 + 摘要）。
#[derive(Debug, Clone, Serialize)]
pub struct BatchReport {
    pub id: i64,
    pub batch_id: String,
    /// scan | clean_ghosts | reindex | legacy_import
    pub batch_type: String,
    pub started_at: Option<String>,
    /// 非空 = 合法完成（崩溃留下的半截批次此项为空）
    pub finished_at: Option<String>,
    /// running | completed | interrupted
    pub status: String,
    /// 摘要 JSON 原文，由前端按需解析展示
    pub summary_json: Option<String>,
}
