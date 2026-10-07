// 导出文件到用户指定目录（第五轮 Batch 31：用户可在设置里配置默认「导出到哪里」）。
//
// 此前导出走浏览器下载（downloadBlob），文件落到浏览器默认下载目录，用户不知道在哪。
// 现在提供 export_file 命令：直接把内容写到用户配置的目录，返回完整路径供前端 Toast 提示。

use std::fs;
use std::path::PathBuf;
use crate::fs_trace;

/// 把文本内容写到 `dir/filename`，返回完整路径。
/// - `dir` 为空时返回错误（前端此时回退到浏览器下载）。
/// - 自动清理文件名中的非法字符，防止路径穿越。
#[tauri::command]
pub async fn export_file(
    dir: String,
    filename: String,
    content: String,
) -> Result<String, String> {
    let trimmed = dir.trim();
    if trimmed.is_empty() {
        return Err("Export directory is empty".to_string());
    }

    let dir_path = PathBuf::from(trimmed);
    fs::create_dir_all(&dir_path)
        .map_err(|e| format!("Failed to create export directory: {}", e))?;

    // 清理文件名里的路径分隔符与 Windows 非法字符
    let safe_name: String = filename
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect();

    let file_path = dir_path.join(&safe_name);
    fs::write(&file_path, content).map_err(|e| format!("Failed to write file: {}", e))?;

    let full = file_path.to_string_lossy().to_string();
    fs_trace!("exported: {}", full);
    Ok(full)
}
