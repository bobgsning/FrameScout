// 幽灵清理的数据模型。
//
// 产品原则：**系统有感知，行动由用户触发**。
// 因此幽灵处理被拆成「预览」与「行动」两步，且每种行动都有对应的反向动作：
//   标记失效（dead） ←→ 回库（restore）
//   真删除（purge）  →  终态，不可回退（所以必须先预览）
// 预览本身不写任何状态、不产生任何事件——它只是把事实摆到用户面前。

use serde::Serialize;

/// 预览列表中的一项。
#[derive(Debug, Clone, Serialize)]
pub struct GhostItem {
    /// 规范化后的路径
    pub path: String,
    /// 该文件仍存的向量行数（视频 = 帧数）。
    /// 展示它，是为了让用户理解「真删除会失去什么」。
    pub frame_count: usize,
    /// 库中记录的最后观察大小（字节）
    pub observed_size: Option<i64>,
    /// 库中记录的最后观察修改时间（epoch 毫秒）
    pub observed_mtime: Option<i64>,
    /// 系统最后一次确认该文件存在的时刻
    pub last_seen_at: Option<String>,
    /// 被标记失效的时刻（未失效则为 None）
    pub dead_at: Option<String>,
    /// 是否已被标记失效（占库但不参与检索）
    pub is_dead: bool,
    /// 此刻磁盘上是否存在
    pub exists_on_disk: bool,
    /// 该文件所在盘根当前不可达（NAS 未挂载 / 移动硬盘未插）（P0-1 / 第三轮 D7）。
    /// true 时前端应整组灰显并禁用 purge——避免误删只是暂时离线的整个盘。
    /// is_dead=true 的文件不受影响（向量在库里，restore/purge 都不依赖磁盘）。
    pub disk_offline: bool,
}

/// 一次幽灵处理动作的结果。
#[derive(Debug, Clone, Serialize)]
pub struct GhostActionResult {
    /// 承载本次动作的批次 id（可用于整批来源筛选）
    pub batch_id: String,
    /// 执行的动作：mark_dead / restore / purge
    pub action: String,
    /// 实际处理的路径
    pub affected_paths: Vec<String>,
    /// 受影响的向量行（帧）总数
    pub affected_frames: usize,
    /// 被跳过的路径（状态不符，例如对未失效的文件执行 restore）
    pub skipped_paths: Vec<String>,
}
