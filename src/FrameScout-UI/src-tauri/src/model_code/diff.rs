// 差异报告的数据模型。
//
// 产品原则：**系统有感知，行动由用户触发**。
// 差异报告只回答「世界和上次比变成了什么样」，绝不替用户决定该索引还是该忽略。
//
// 四组分类缺一不可：
//   - 新增：磁盘有、库里没有
//   - 修改：路径在，但 mtime 或大小变了（已过 2s 宽容阈值）
//   - 消失：库里有、磁盘没了（本次扫描目录内）
//   - 需要索引但无向量：**常驻第四组**，专门防止「索引失败但 observed_* 已写」
//     导致文件被静默永久跳过，是「失败可重试」的最低契约。

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    /// 磁盘有、库里没有
    New,
    /// mtime 或大小变了
    Modified,
    /// 库里有、磁盘没了
    Missing,
    /// 有锚点但库里没有有效向量（索引失败待重试）
    Unindexed,
}

impl DiffKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DiffKind::New => "new",
            DiffKind::Modified => "modified",
            DiffKind::Missing => "missing",
            DiffKind::Unindexed => "unindexed",
        }
    }
}

/// 差异清单中的一项。
#[derive(Debug, Clone, Serialize)]
pub struct DiffItem {
    /// 规范化后的路径
    pub path: String,
    pub kind: DiffKind,
    /// 库中记录的观察值（新增项则为 None）
    pub observed_mtime: Option<i64>,
    pub observed_size: Option<i64>,
    /// 磁盘当前值（消失/待索引项则为 None）
    pub disk_mtime: Option<i64>,
    pub disk_size: Option<i64>,
    /// 库中已有的向量行数（视频 = 帧数）
    pub frame_count: usize,
    pub last_seen_at: Option<String>,
}

/// 重索引失败项。
#[derive(Debug, Clone, Serialize)]
pub struct ReindexFailure {
    pub path: String,
    pub error: String,
}

/// 重索引结果。
#[derive(Debug, Clone, Serialize)]
pub struct ReindexResult {
    pub batch_id: String,
    /// 成功完成重索引的路径
    pub succeeded: Vec<String>,
    /// 失败的路径（已写入 `reindex_failed` 事件，仍留在第四组待重试）
    pub failed: Vec<ReindexFailure>,
    /// 被替换掉的旧帧行总数
    pub replaced_frames: usize,
    /// 新写入的帧行总数
    pub new_frames: usize,
}

/// 一次差异报告。
#[derive(Debug, Clone, Serialize)]
pub struct DiffReport {
    /// 骨架：最近一次合法完成的扫描批次 id（没有则为 None）
    pub baseline_batch_id: Option<String>,
    /// 本次预扫描覆盖的根目录
    pub root: String,
    /// 磁盘上扫描到的媒体文件数
    pub scanned_files: usize,
    /// 疑似外部同步（大量文件集体变化）——只提示，由用户判断
    pub mass_change_suspected: bool,
    pub new_count: usize,
    pub modified_count: usize,
    pub missing_count: usize,
    pub unindexed_count: usize,
    pub items: Vec<DiffItem>,
}
