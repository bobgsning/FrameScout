use std::process::Child;
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
    /// 扫描/索引暂停标志（M2'''：暂停/继续）。
    ///
    /// `pause_scan` 置 true，`process_file_paths_internal` 每批开头发现暂停则
    /// 原地轮询等待（不推进、不退出），`resume_scan` 置 false 后继续。
    pub pause_flag: Arc<AtomicBool>,
    /// 数据库文件路径（P2-5：备份/恢复命令需要知道 .db 文件位置）。
    pub db_path: Mutex<String>,
    /// AI Worker 子进程句柄（分段重启用）。
    ///
    /// DirectML 后端在连续大量推理后会资源耗尽崩溃（`UnicodeDecodeError`），
    /// 索引 OCR 大量文件时需定期重启 worker 来释放 GPU 资源（见 `index_cmd.rs`
    /// 的 `OCR_RESTART_THRESHOLD` 分段逻辑）。启动时由 `lib.rs` spawn 后放入，
    /// 退出时从中取出 kill。
    pub worker_process: Mutex<Option<Child>>,
    /// 启动期自动清理的汇总报告（M0''' 告知：把「系统删了什么」回传给前端 Toast）。
    pub startup_cleanup: Mutex<StartupCleanupReport>,
}

/// 启动期自动清理的汇总（M0''' 第 2 项：自动清理要告知）。
///
/// `init_db_and_load_memory` 在启动时按用户策略裁剪事件 / 冷备份 / 残行，
/// 把删了什么、多少条记录进此结构，由 `get_startup_cleanup_report` 命令回传前端，
/// 前端在挂载后 Toast 告知用户「系统替你清了什么」。
#[derive(serde::Serialize, Default, Clone)]
pub struct StartupCleanupReport {
    /// 裁剪掉的过期 file_events 条数
    pub pruned_events: usize,
    /// 裁剪掉的过期 batches 条数
    pub pruned_batches: usize,
    /// 裁剪掉的 reindex 冷备份条数
    pub pruned_reindex_backups: usize,
    /// 清理掉的残行（既无有效 BLOB 也无有效 JSON）涉及的文件数
    pub cleaned_residual_rows: usize,
    /// 第八轮：这次清理发生的时间（UTC ISO）。
    /// 没有它，设置页只能说「清了什么」，说不出「什么时候清的」—— 账簿缺一个维度。
    /// 未发生任何清理时为 None。
    pub cleaned_at: Option<String>,
}
