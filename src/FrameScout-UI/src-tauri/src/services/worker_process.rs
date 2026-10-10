// 负责扫描并杀死残留的僵尸进程、跨平台定位可执行文件，以及后台静默拉起 ai_worker 进程。

use std::env;
use std::path::{Path, PathBuf};
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
        // No packaged worker (e.g. `tauri dev` without Nuitka): run it from the
        // Python source tree instead of failing.
        return spawn_python_worker();
    }

    println!("👻 Spawning AI Worker: {:?}", ai_path);

    // 3. 后台启动子进程 (Windows 下隐藏控制台窗口)
    let mut cmd = Command::new(&ai_path);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    cmd.spawn().expect("❌ Failed to spawn ai_worker")
}

/// Development fallback: launch the worker straight from Python source
/// (`src/inference-worker/main.py`), so no Nuitka/PyInstaller build is needed.
///
/// Interpreter lookup: `FRAMESCOUT_WORKER_PYTHON` if set, otherwise the
/// worker's own virtualenv (`.venv`). The child inherits this process's console,
/// so the worker's logs show up in the terminal running `tauri dev`.
fn spawn_python_worker() -> Child {
    let worker_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("inference-worker");

    let python: PathBuf = match env::var_os("FRAMESCOUT_WORKER_PYTHON") {
        Some(p) => PathBuf::from(p),
        None if cfg!(target_os = "windows") => {
            worker_dir.join(".venv").join("Scripts").join("python.exe")
        }
        None => worker_dir.join(".venv").join("bin").join("python"),
    };

    if !worker_dir.join("main.py").exists() {
        panic!("Cannot find ai_worker.exe and no Python worker source at {:?}", worker_dir);
    }
    if !python.exists() {
        panic!(
            "Cannot find ai_worker.exe, and the Python interpreter {:?} does not exist. \
             Create the venv in src/inference-worker (python -m venv .venv) and install \
             requirements.txt, or set FRAMESCOUT_WORKER_PYTHON.",
            python
        );
    }

    println!("👻 Spawning AI Worker from Python source: {:?} main.py", python);

    Command::new(&python)
        .args(["-u", "main.py"])
        .current_dir(&worker_dir)
        .spawn()
        .expect("❌ Failed to spawn Python AI worker")
}

/// Stop the worker and everything it started. On Windows a venv's `python.exe`
/// (and Nuitka onefile builds) run the real work in a child process, so killing
/// only the direct child can leave the worker holding port 16666.
pub fn kill_ai_worker(child: &mut Child) {
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &child.id().to_string()])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
    }
    let _ = child.kill();
}
