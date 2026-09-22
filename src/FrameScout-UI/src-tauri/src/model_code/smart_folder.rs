use serde::Serialize;

#[derive(Serialize)]
pub struct SmartFolder {
    pub id: i64,
    pub name: String,
    pub query_text: String,
    pub use_vector: bool,
    pub use_ocr: bool,
    pub use_note: bool,
    pub use_filename: bool,
    pub match_count: usize,
}