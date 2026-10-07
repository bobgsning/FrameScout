use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};
use rusqlite::Connection;
use crate::storage::FlatVectorMatrix;
use crate::license::guard::TrialGuard;

pub struct AppState {
    pub db_conn: Mutex<Connection>,
    pub memory_db: RwLock<FlatVectorMatrix>,
    pub trial_guard: Mutex<TrialGuard>,
    /// 扫描/索引取消标志（P0-6 / 第三轮 B13）。
    ///
    /// `cancel_scan` 命令把它置 true，`process_file_paths_internal` 每批开头检查，
    /// 命中即停止后续批处理并返回已完成的数量。AtomicBool + Arc 让命令线程
    /// 与扫描线程无需持锁即可通信。
    ///
    /// 启动时置 false；每次 `scan_folder` / `index_files` 入口也会先 reset 为 false。
    pub cancel_flag: Arc<AtomicBool>,
    /// 数据库文件路径（P2-5：备份/恢复命令需要知道 .db 文件位置）。
    pub db_path: Mutex<String>,
}
