// Copyright 2026 AetherFlow Labs
// All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// =========================================================================
//  FrameScout — Offline AI Search — Rust/Tauri Core
//  Version: 3.2.0
//  License: Apache-2.0 (core) / Proprietary (license verification)
//
//  库入口：所有后端模块在此聚合，桌面端与移动端共用同一个 run()。
// =========================================================================

mod commands;
mod constants;
mod license;
mod logging;
mod model_code;
mod proto;
mod services;
mod storage;
mod time_util;

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};

use license::TrialGuard;
use model_code::AppState;
use tauri::RunEvent;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 1. 启动 Python AI Worker 推理子进程
    let mut ai_process = services::spawn_ai_worker();

    // 2. 初始化 SQLite 数据库并载入内存向量矩阵
    let (db_conn, memory_db) = storage::init_db_and_load_memory();

    // P2-5：数据库文件路径（备份命令需要）
    let db_path_string = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("FrameScout-Offline_AI_Search-Global")
        .join("framescout-offline_ai_search-global.db")
        .to_string_lossy()
        .to_string();

    // 3. 构建并运行 Tauri 应用程序
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            db_conn: Mutex::new(db_conn),
            memory_db: RwLock::new(memory_db),
            trial_guard: Mutex::new(TrialGuard::new()),
            // 取消标志共享给扫描线程与 cancel_scan 命令（P0-6）。
            cancel_flag: Arc::new(AtomicBool::new(false)),
            db_path: Mutex::new(db_path_string),
        })
        .setup(|app| {
            let app_handle = app.handle().clone();
            // 在独立的后台线程中轮询检测 AI 引擎就绪状态
            std::thread::spawn(move || {
                services::wait_for_engine_and_notify(app_handle);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 索引扫描命令
            commands::index_cmd::scan_folder,
            commands::index_cmd::index_files,
            commands::index_cmd::cancel_scan,

            // OCR 识别命令
            commands::ocr_cmd::run_ocr_for_selected_files,

            // 搜索与引擎状态命令
            commands::search_cmd::search_images,
            commands::search_cmd::search_by_image,
            commands::search_cmd::search_text,
            commands::search_cmd::insert_text_entry,
            commands::search_cmd::search_text_entries,
            commands::search_cmd::ping_engine,

            // 视觉聚类命令
            commands::cluster_cmd::cluster_similar_images,

            // 智能文件夹命令
            commands::smart_folder_cmd::save_smart_folder,
            commands::smart_folder_cmd::update_smart_folder,
            commands::smart_folder_cmd::get_smart_folders,
            commands::smart_folder_cmd::refresh_smart_folder_count,
            commands::smart_folder_cmd::execute_smart_folder,
            commands::smart_folder_cmd::delete_smart_folder,
            // P2-2：RRF 跨通道融合
            commands::search_unified_cmd::search_unified,
            // P2-5：备份/恢复/完整性校验
            commands::backup_cmd::backup_database,
            commands::backup_cmd::verify_integrity,
            commands::backup_cmd::list_backups,
            commands::backup_cmd::delete_backup,
            // P2-4：时间轴 + 去年今天
            commands::report_cmd::list_timeline,
            commands::report_cmd::last_year_today_count,
            commands::report_cmd::list_files_by_day,
            // P2-9：ffmpeg 片段剪切
            commands::ffmpeg_cmd::probe_ffmpeg,
            commands::ffmpeg_cmd::cut_video_clip,

            // 导出到指定目录
            commands::export_cmd::export_file,

            // 文件管理与元数据维护命令
            commands::file_cmd::update_note,
            commands::file_cmd::list_all_files,
            commands::file_cmd::get_all_files,

            // 幽灵清理命令（先预览、后行动）
            commands::ghost_cmd::preview_ghosts,
            commands::ghost_cmd::apply_ghost_action,

            // 差异报告命令（只感知，行动由用户触发后走 index_files / apply_ghost_action）
            commands::diff_cmd::scan_diff,
            commands::reindex_cmd::reindex_files,

            // 扫描报告 / 活动记录
            commands::report_cmd::list_scan_reports,

            // 纯文本通道（文本文件直接入库 + 条目管理）
            commands::text_cmd::ingest_text_files,
            commands::text_cmd::list_text_entries,
            commands::text_cmd::delete_text_entries,

            // 授权相关命令
            commands::license_cmd::activate_pro_license,
            commands::license_cmd::get_license_status,
        ])
        .build(tauri::generate_context!())
        .expect("Tauri Build Fail");

    // 4. 监听退出事件：应用关闭时自动杀掉 AI Worker 子进程
    app.run(move |_, event| {
        if let RunEvent::Exit = event {
            services::kill_ai_worker(&mut ai_process);
        }
    });
}
