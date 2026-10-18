// ffmpeg 片段剪切（P2-9 / 第三轮「导出 trio：ffmpeg 片段剪切」）。
//
// 视频搜索命中后，用户可能想「把命中的那 5 秒切出来」发给同事。
// 本命令探测 ffmpeg 是否可用，可用则调用 `ffmpeg -ss <start> -t <duration> -i <input> -c copy <output>`
// 切出片段（无重编码，极快），不可用则返回提示让前端禁用按钮。

use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::State;
use crate::error::{
    ec1, ERR_EXPORT_DIR_CREATE_FAILED, ERR_FFMPEG_CUT_FAILED, ERR_FFMPEG_INVOKE_FAILED,
    ERR_FFMPEG_SOURCE_NOT_FOUND,
};
use crate::model_code::AppState;

/// 返回要执行的 ffmpeg 可执行文件：优先打包在 ai_worker 同级的 ffmpeg.exe，否则回退 PATH。
///
/// 与 Python 侧 `video_extractor.py::_resolve_ffmpeg` 保持同一语义：打包发布包把
/// ffmpeg.exe 放在 ai_worker.exe 同级，切片段与兜底解码共用同一处，不再依赖用户 PATH。
fn ffmpeg_exe() -> PathBuf {
    if let Some(worker_dir) = crate::services::worker_process::ai_worker_dir() {
        let bundled = worker_dir.join("ffmpeg.exe");
        if bundled.is_file() {
            return bundled;
        }
    }
    PathBuf::from("ffmpeg") // 开发期兜底：依赖 PATH
}

/// 探测 ffmpeg 是否可用（打包的 ffmpeg.exe 优先，PATH 兜底）。
#[tauri::command]
pub async fn probe_ffmpeg() -> Result<bool, String> {
    let result = Command::new(ffmpeg_exe()).arg("-version").output();
    // 只判「能 spawn」不够：损坏但可启动的 ffmpeg 会返回非 0 退出码，须一并校验。
    Ok(matches!(result, Ok(output) if output.status.success()))
}

/// 切出视频片段（无重编码，极快）。
///
/// - `input_path`：源视频路径
/// - `output_path`：输出路径（与源同容器后缀，由前端按源扩展名生成）
/// - `start_sec`：片段起始秒
/// - `duration_sec`：片段时长（默认 5 秒）
#[tauri::command]
pub async fn cut_video_clip(
    _state: State<'_, AppState>,
    input_path: String,
    output_path: String,
    start_sec: f32,
    duration_sec: Option<f32>,
) -> Result<String, String> {
    if !Path::new(&input_path).exists() {
        return Err(ec1(ERR_FFMPEG_SOURCE_NOT_FOUND, input_path));
    }

    let dur = duration_sec.unwrap_or(5.0);

    // 输出目录可能尚未存在（导出目录未设置时回退源视频同目录），先创建再让 ffmpeg 写入。
    if let Some(parent) = Path::new(&output_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| ec1(ERR_EXPORT_DIR_CREATE_FAILED, e))?;
        }
    }

    // 用 -ss before -i（seek before input，极快）+ -c copy（无重编码）
    let output = Command::new(ffmpeg_exe())
        .args([
            "-y",  // 覆盖已存在的输出
            "-ss", &format!("{:.1}", start_sec),
            "-t", &format!("{:.1}", dur),
            "-i", &input_path,
            "-c", "copy",
            &output_path,
        ])
        .output()
        .map_err(|e| ec1(ERR_FFMPEG_INVOKE_FAILED, e))?;

    if output.status.success() {
        Ok(output_path)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(ec1(ERR_FFMPEG_CUT_FAILED, stderr.chars().take(500).collect::<String>()))
    }
}
