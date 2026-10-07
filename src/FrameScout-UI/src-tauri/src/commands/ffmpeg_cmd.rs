// ffmpeg 片段剪切（P2-9 / 第三轮「导出 trio：ffmpeg 片段剪切」）。
//
// 视频搜索命中后，用户可能想「把命中的那 5 秒切出来」发给同事。
// 本命令探测 ffmpeg 是否可用，可用则调用 `ffmpeg -ss <start> -t <duration> -i <input> -c copy <output>`
// 切出片段（无重编码，极快），不可用则返回提示让前端禁用按钮。

use std::process::Command;
use std::path::Path;
use tauri::State;
use crate::model_code::AppState;

/// 探测 ffmpeg 是否在 PATH 中可用。
#[tauri::command]
pub async fn probe_ffmpeg() -> Result<bool, String> {
    let result = Command::new("ffmpeg").arg("-version").output();
    Ok(result.is_ok())
}

/// 切出视频片段（无重编码，极快）。
///
/// - `input_path`：源视频路径
/// - `output_path`：输出 .mp4 路径
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
        return Err(format!("Source file not found: {}", input_path));
    }

    let dur = duration_sec.unwrap_or(5.0);

    // 用 -ss before -i（seek before input，极快）+ -c copy（无重编码）
    let output = Command::new("ffmpeg")
        .args([
            "-y",  // 覆盖已存在的输出
            "-ss", &format!("{:.1}", start_sec),
            "-t", &format!("{:.1}", dur),
            "-i", &input_path,
            "-c", "copy",
            &output_path,
        ])
        .output()
        .map_err(|e| format!("ffmpeg invocation failed: {}. Please make sure ffmpeg is installed and available in PATH.", e))?;

    if output.status.success() {
        Ok(output_path)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("ffmpeg cut failed: {}", stderr.chars().take(500).collect::<String>()))
    }
}
