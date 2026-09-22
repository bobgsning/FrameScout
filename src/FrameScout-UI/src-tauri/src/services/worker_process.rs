// 负责扫描并杀死残留的僵尸进程、跨平台定位可执行文件，以及后台静默拉起 ai_worker 进程。

use std::env;
use std::process::{Child, Command};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

pub fn spawn_ai_worker() -> Child {
    // 1. 清理可能残留的旧 worker 进程
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "ai_worker.exe", "/T"])
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("pkill")
            .arg("-f")
            .arg("ai_worker")
            .output();
    }

    // 2. 寻找 worker 可执行文件路径
    let current_exe = env::current_exe().unwrap();
    let exe_dir = current_exe.parent().unwrap();
    let ai_worker_name = if cfg!(target_os = "windows") {
        "ai_worker.exe"
    } else {
        "ai_worker"
    };

    let mut ai_path = exe_dir.join("ai_worker").join(ai_worker_name);

    if !ai_path.exists() {
        let project_src = exe_dir
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(exe_dir);
        ai_path = project_src.join("bin").join("ai_worker").join(ai_worker_name);
    }

    if !ai_path.exists() {
        panic!(
            "Cannot find ai_worker. Expected at {:?} or {:?}",
            exe_dir.join("ai_worker").join(ai_worker_name),
            ai_path
        );
    }

    println!("👻 Spawning AI Worker: {:?}", ai_path);

    // 3. 后台启动子进程 (Windows 下隐藏控制台窗口)
    let mut cmd = Command::new(&ai_path);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    cmd.spawn().expect("❌ Failed to spawn ai_worker")
}