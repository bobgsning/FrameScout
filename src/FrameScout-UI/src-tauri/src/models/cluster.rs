use serde::Serialize;

#[derive(Serialize)]
pub struct ClusterGroup {
    pub group_id: usize,
    pub representative_path: String,
    pub member_paths: Vec<String>,
}