// 负责 Pro 版本的授权激活与查询授权状态。

use tauri::State;
use crate::constants::FREE_TRIAL_LIMIT;
use crate::models::AppState;

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
    fs::write(lic_path, content).map_err(|e| e.to_string())?;

    let mut guard = state.trial_guard.lock().unwrap();
    guard.is_pro = true;
    guard.user_email = email.clone();

    Ok(format!("💎 Successfully activated FrameScout Pro for {}!", email))
}

#[cfg(feature = "pro")]
#[tauri::command]
pub async fn get_license_status(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let guard = state.trial_guard.lock().unwrap();
    Ok(serde_json::json!({
        "is_pro": guard.is_pro,
        "email": guard.user_email,
        "limit": FREE_TRIAL_LIMIT,
    }))
}

#[cfg(not(feature = "pro"))]
#[tauri::command]
pub async fn activate_pro_license(
    _state: State<'_, AppState>,
    _email: String,
    _license_key: String,
) -> Result<String, String> {
    Err("Pro activation is not available in the open source edition.".to_string())
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