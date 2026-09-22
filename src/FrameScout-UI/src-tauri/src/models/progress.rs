use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub status: String,
    pub file_path: String,
    pub current: usize,
    pub total: usize,
    pub new_files: Vec<String>,
}