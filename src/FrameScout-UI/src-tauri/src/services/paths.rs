// 解析 AI worker 及其模型目录 / 语言目录 manifest 的位置。
//
// 与 `worker_process.rs::spawn_ai_worker` 的定位逻辑保持一致，确保 Rust 侧
// 读到的 `ocr_manifest.json` 与 `models/ocr/` 和 Python worker 的
// `config.py::get_base_path()` 指向同一份文件，避免两份事实漂移。
//
// 三种部署形态：
//   1. 打包：`<exe_dir>/ai_worker/`（ai_worker.exe 与其 models/ 同目录）
//   2. 部署到 bin：`<src-tauri>/bin/ai_worker/`（build_nuitka 产物落点）
//   3. 源码开发：`<repo>/src/inference-worker/`（main.py 直接运行）

use std::env;
use std::path::{Path, PathBuf};

fn worker_exe_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "ai_worker.exe"
    } else {
        "ai_worker"
    }
}

/// 定位 worker 目录（包含 ai_worker.exe 或 main.py 的目录）。
pub fn resolve_worker_dir() -> PathBuf {
    let current_exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    let exe_dir = current_exe.parent().unwrap_or_else(|| Path::new("."));

    // 1. 打包形态：exe 同级的 ai_worker/
    let packaged = exe_dir.join("ai_worker").join(worker_exe_name());
    if packaged.exists() {
        return exe_dir.join("ai_worker");
    }

    // 2. 部署到 src-tauri/bin/ai_worker/（`tauri dev` + build_nuitka 部署）
    let project_src = exe_dir
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or_else(|| Path::new("."));
    let bin_deployed = project_src.join("bin").join("ai_worker").join(worker_exe_name());
    if bin_deployed.exists() {
        return project_src.join("bin").join("ai_worker");
    }

    // 3. 源码开发：<repo>/src/inference-worker/
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("inference-worker")
}

/// OCR 语言目录 manifest（单一事实源）路径。
pub fn resolve_ocr_manifest_path() -> PathBuf {
    resolve_worker_dir().join("ocr_manifest.json")
}

/// OCR 多语言模型目录（models/ocr/）。
pub fn resolve_ocr_dir() -> PathBuf {
    resolve_worker_dir().join("models").join("ocr")
}
