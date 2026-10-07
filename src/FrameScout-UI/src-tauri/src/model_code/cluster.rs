use serde::Serialize;

#[derive(Serialize)]
pub struct ClusterGroup {
    pub group_id: usize,
    pub representative_path: String,
    pub member_paths: Vec<String>,
}

/// 聚类结果（债单 B8：告知被截断，不再静默消失）。
#[derive(Serialize)]
pub struct ClusterResult {
    pub groups: Vec<ClusterGroup>,
    /// 因「单帧簇 ≤50」限制而被丢弃的独特帧数量。
    pub truncated_single_frames: usize,
}