// Copyright 2026 AetherFlow Labs Inc. All rights reserved.
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
//  Version: 3.1.0
//  License: Apache-2.0 (core) / Proprietary (license verification)
//
//  库入口：所有后端模块在此聚合，桌面端与移动端共用同一个 run()。
// =========================================================================

mod commands;
mod constants;
mod license;
mod models;
mod proto;
mod services;
mod storage;

use std::sync::{Mutex, RwLock};

use license::TrialGuard;
use models::AppState;
use tauri::RunEvent;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 1. 启动 Python AI Worker 推理子进程
    let mut ai_process = services::spawn_ai_worker();

    // 2. 初始化 SQLite 数据库并载入内存向量矩阵
    let (db_conn, memory_db) = storage::init_db_and_load_memory();

    // 3. 构建并运行 Tauri 应用程序
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            db_conn: Mutex::new(db_conn),
            memory_db: RwLock::new(memory_db),
            trial_guard: Mutex::new(TrialGuard::new()),
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

            // OCR 识别命令
            commands::ocr_cmd::run_ocr_for_selected_files,

            // 搜索与引擎状态命令
            commands::search_cmd::search_images,
            commands::search_cmd::search_by_image,
            commands::search_cmd::ping_engine,

            // 视觉聚类命令
            commands::cluster_cmd::cluster_similar_images,

            // 智能文件夹命令
            commands::smart_folder_cmd::save_smart_folder,
            commands::smart_folder_cmd::get_smart_folders,
            commands::smart_folder_cmd::execute_smart_folder,
            commands::smart_folder_cmd::delete_smart_folder,

            // 文件管理与元数据维护命令
            commands::file_cmd::update_note,
            commands::file_cmd::clean_ghosts,
            commands::file_cmd::list_all_files,
            commands::file_cmd::get_all_files,

            // 授权相关命令
            commands::license_cmd::activate_pro_license,
            commands::license_cmd::get_license_status,
        ])
        .build(tauri::generate_context!())
        .expect("Tauri Build Fail");

    // 4. 监听退出事件：应用关闭时自动杀掉 AI Worker 子进程
    app.run(move |_, event| {
        if let RunEvent::Exit = event {
            let _ = ai_process.kill();
        }
    });
}
