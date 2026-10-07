// 模块聚合与重导出

pub mod progress;
pub mod cluster;
pub mod smart_folder;
pub mod search;
pub mod state;
pub mod ghost;
pub mod diff;
pub mod report;
pub mod frame;
pub mod text;

pub use progress::ProgressPayload;
pub use cluster::{ClusterGroup, ClusterResult};
pub use smart_folder::SmartFolder;
pub use search::{
    OcrLineHit, PagedResponse, SearchResult, TextEntryHit, TextEntryItem, TextEntryListResponse,
    TextEntryPagedResponse, TextPagedResponse, TextSearchHit,
};
pub use state::AppState;
pub use ghost::{GhostActionResult, GhostItem};
pub use diff::{DiffItem, DiffKind, DiffReport, ReindexFailure, ReindexResult};
pub use report::BatchReport;
pub use frame::{EncodedFrame, OcrLine};
pub use text::{TextIngestFailure, TextIngestResult};
