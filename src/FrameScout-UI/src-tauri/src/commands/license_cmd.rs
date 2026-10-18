// 负责 Pro 版本的授权激活与查询授权状态。

use tauri::State;
use crate::constants::FREE_TRIAL_LIMIT;
// 授权码只在对应构建里用到，故按 feature 门控，避免未使用导入告警。
#[cfg(feature = "pro")]
use crate::error::{ec1, ERR_LICENSE_WRITE_FAILED, MSG_LICENSE_ACTIVATED};
#[cfg(not(feature = "pro"))]
use crate::error::{ec, ERR_LICENSE_EDITION_UNAVAILABLE};
use crate::model_code::AppState;

#[cfg(feature = "pro")]
use crate::license::verifier;
#[cfg(feature = "pro")]
use std::fs;

#[cfg(feature = "pro")]
#[tauri::command]
pub async fn activate_pro_license(
    state: State<'_, AppState>,
    email: String,
    license_key: String,
) -> Result<String, String> {
    verifier::verify_license_key(&email, &license_key)?;

    let app_data_dir = dirs::data_local_dir().unwrap().join("FrameScout-Offline_AI_Search-Global");
    let lic_path = app_data_dir.join("framescout.lic");
    let content = format!("{}\n{}", email.trim().to_lowercase(), license_key.trim());
    fs::write(lic_path, content).map_err(|e| ec1(ERR_LICENSE_WRITE_FAILED, e))?;

    let mut guard = state.trial_guard.lock().unwrap();
    guard.is_pro = true;
    guard.user_email = email.clone();

    Ok(ec1(MSG_LICENSE_ACTIVATED, email))
}

#[cfg(feature = "pro")]
#[tauri::command]
pub async fn get_license_status(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let guard = state.trial_guard.lock().unwrap();
    let mut json = serde_json::json!({
        "is_pro": guard.is_pro,
        "email": guard.user_email,
        "limit": FREE_TRIAL_LIMIT,
        "expiry": null,
        "version": null,
    });

    // 激活后从 .lic 解析「到期时间戳 + 锁定版本」，供前端友好展示（验签已通过）。
    if guard.is_pro {
        let app_data_dir = dirs::data_local_dir().unwrap().join("FrameScout-Offline_AI_Search-Global");
        let lic_path = app_data_dir.join("framescout.lic");
        if let Ok(content) = fs::read_to_string(lic_path) {
            let lines: Vec<&str> = content.lines().collect();
            if lines.len() >= 2 {
                if let Some((expiry, version)) = verifier::parse_license_info(lines[1].trim()) {
                    json["expiry"] = serde_json::json!(expiry);
                    json["version"] = serde_json::json!(version);
                }
            }
        }
    }

    Ok(json)
}

#[cfg(not(feature = "pro"))]
#[tauri::command]
pub async fn activate_pro_license(
    _state: State<'_, AppState>,
    _email: String,
    _license_key: String,
) -> Result<String, String> {
    Err(ec(ERR_LICENSE_EDITION_UNAVAILABLE))
}

#[cfg(not(feature = "pro"))]
#[tauri::command]
pub async fn get_license_status(
    _state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "is_pro": false,
        "email": "",
        "limit": FREE_TRIAL_LIMIT,
    }))
}