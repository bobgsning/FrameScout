// OCR 多语言模型管理命令：列出可用语言、下载、校验、删除。
//
// 语言目录由 `ocr_manifest.json` 单一事实源描述（语言码 / 分层 / rec 模型文件名 /
// 下载 URL / SHA256）。Rust 侧通过 services/paths.rs 解析与 Python worker 相同的
// 目录，读同一份 manifest，避免两份硬编码漂移。
//
// 分层（tier）：
//   builtin_wheel   — 随 rapidocr_onnxruntime wheel 内置（en/ch_sim），不可删
//   builtin_bundle  — 随安装包内置（ch_tra/ja），可删
//   downloadable    — 用户按需下载（ru/ko/de/fr/es），可删
//   planned         — 规划占位（pt/it/pl/tr/vi/id），仅显示、不可交互
//
// 下载：官方镜像优先、自建镜像兜底（urls 有序列表），HTTP Range 断点续传（.part），
// 完成后 SHA256 校验，通过后原子重命名；下载前做磁盘可用空间检查。

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Deserialize;
use tauri::Emitter;

use crate::services::paths::{resolve_ocr_dir, resolve_ocr_manifest_path};
use crate::fs_trace;

// ------------------------------------------------------------------
// Manifest 结构（只解析需要的最小字段）
// ------------------------------------------------------------------
#[derive(Deserialize)]
struct Manifest {
    #[serde(default)]
    languages: Vec<ManifestLanguage>,
}

#[derive(Deserialize, Clone)]
struct ManifestLanguage {
    code: String,
    native: String,
    tier: String,
    #[serde(default)]
    deletable: bool,
    #[serde(default)]
    rec_model: Option<String>,
    #[serde(default)]
    rec_dict: Option<String>,
    #[serde(default)]
    urls: Vec<String>,
    #[serde(default)]
    dict_urls: Vec<String>,
    #[serde(default)]
    sha256: Option<String>,
}

// ------------------------------------------------------------------
// 前端 DTO
// ------------------------------------------------------------------
#[derive(serde::Serialize)]
pub struct OcrLanguageInfo {
    pub code: String,
    pub native: String,
    pub tier: String,
    pub deletable: bool,
    /// 该语言的识别模型当前是否可用（内置恒可用；可下载/内置随包需文件存在）
    pub available: bool,
}

#[derive(serde::Serialize, Clone)]
pub struct OcrDownloadProgress {
    pub language: String,
    pub downloaded: u64,
    pub total: u64,
    pub percent: u32,
}

// ------------------------------------------------------------------
// 内部工具
// ------------------------------------------------------------------
fn load_manifest() -> Result<Vec<ManifestLanguage>, String> {
    let path = resolve_ocr_manifest_path();
    if !path.exists() {
        return Err(format!("OCR language manifest not found: {}", path.display()));
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read OCR manifest: {}", e))?;
    let manifest: Manifest = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse OCR manifest: {}", e))?;
    Ok(manifest.languages)
}

fn find_language(code: &str) -> Result<ManifestLanguage, String> {
    load_manifest()?
        .into_iter()
        .find(|l| l.code == code)
        .ok_or_else(|| format!("Unknown OCR language: {}", code))
}

fn model_dir_for(code: &str) -> PathBuf {
    resolve_ocr_dir().join(code)
}

fn model_available(lang: &ManifestLanguage) -> bool {
    match lang.tier.as_str() {
        "builtin_wheel" => true,
        "builtin_bundle" | "downloadable" => match &lang.rec_model {
            Some(rec) => model_dir_for(&lang.code).join(rec).is_file(),
            None => false,
        },
        _ => false,
    }
}

/// 计算文件 SHA256（十六进制小写）。
fn sha256_file(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file = fs::File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// 检查目标所在磁盘是否有足够可用空间。
fn check_disk_space(dest: &Path, needed: u64) -> Result<(), String> {
    if needed == 0 {
        return Ok(());
    }
    use sysinfo::Disks;
    let disks = Disks::new_with_refreshed_list();
    let anchor = dest.parent().unwrap_or(dest);
    let target = fs::canonicalize(anchor).unwrap_or_else(|_| anchor.to_path_buf());
    let target_str = target.to_string_lossy().to_lowercase();

    for disk in disks.list() {
        let mount = disk.mount_point().to_string_lossy().to_lowercase();
        if !mount.is_empty() && target_str.starts_with(&mount) {
            if disk.available_space() < needed {
                return Err(format!(
                    "Insufficient disk space: need {} bytes, {} available",
                    needed,
                    disk.available_space()
                ));
            }
            return Ok(());
        }
    }
    Ok(()) // 未匹配到盘符，跳过检查（不阻塞下载）
}

/// 从单个 URL 流式下载到 dest，支持断点续传与 SHA256 校验。
async fn download_from(
    app: &tauri::AppHandle,
    code: &str,
    url: &str,
    dest: &Path,
    sha256: Option<&str>,
) -> Result<(), String> {
    let part = PathBuf::from(format!("{}.part", dest.display()));
    let already = fs::metadata(&part).map(|m| m.len()).unwrap_or(0);

    let client = reqwest::Client::new();
    let mut req = client.get(url);
    if already > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={}-", already));
    }
    let resp = req.send().await.map_err(|e| format!("Download failed: {}", e))?;

    // 计算总量：206 部分内容 = 已下载 + Content-Length；200 完整 = Content-Length
    let total = match resp.status() {
        reqwest::StatusCode::PARTIAL_CONTENT => resp.content_length().map(|l| already + l),
        _ => resp.content_length(),
    }
    .unwrap_or(0);

    // 磁盘空间检查（在写盘前）
    check_disk_space(dest, total.saturating_sub(already))?;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&part)
        .map_err(|e| format!("Failed to create {}: {}", part.display(), e))?;

    let mut downloaded = already;
    let mut resp = resp;
    let mut last_emit = Instant::now() - Duration::from_secs(1);
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        // 进度节流：约每 200ms 发一次事件，避免刷屏
        if last_emit.elapsed() >= Duration::from_millis(200) {
            let percent = if total > 0 { (downloaded * 100 / total) as u32 } else { 0 };
            let _ = app.emit(
                "ocr-download-progress",
                OcrDownloadProgress {
                    language: code.to_string(),
                    downloaded,
                    total,
                    percent,
                },
            );
            last_emit = Instant::now();
        }
    }

    // 收尾进度（100%）
    let _ = app.emit(
        "ocr-download-progress",
        OcrDownloadProgress {
            language: code.to_string(),
            downloaded,
            total: if total > 0 { total } else { downloaded },
            percent: 100,
        },
    );

    // SHA256 完整性校验
    if let Some(expected) = sha256 {
        let actual = sha256_file(&part)?;
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = fs::remove_file(&part);
            return Err(format!("SHA256 mismatch: expected {}, got {}", expected, actual));
        }
    }

    fs::rename(&part, dest).map_err(|e| format!("Failed to finalize download: {}", e))?;
    fs_trace!("OCR model downloaded: {}", dest.display());
    Ok(())
}

/// 按序尝试多个 URL（官方优先，自建镜像兜底）。
async fn download_file(
    app: &tauri::AppHandle,
    code: &str,
    urls: &[String],
    dest: &Path,
    sha256: Option<&str>,
) -> Result<(), String> {
    let mut last_err = None;
    for url in urls {
        match download_from(app, code, url, dest, sha256).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                println!("⚠️ Download failed, trying next source: {}\n   {}", url, e);
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| "No download URLs configured".to_string()))
}

// ------------------------------------------------------------------
// 命令
// ------------------------------------------------------------------
/// 列出所有 OCR 语言及其可用性状态（内置 / 可下载 / 规划，含当前是否已就绪）。
#[tauri::command]
pub async fn list_ocr_languages() -> Result<Vec<OcrLanguageInfo>, String> {
    let languages = load_manifest()?;
    let infos = languages
        .iter()
        .map(|l| OcrLanguageInfo {
            code: l.code.clone(),
            native: l.native.clone(),
            tier: l.tier.clone(),
            deletable: l.deletable,
            available: model_available(l),
        })
        .collect();
    Ok(infos)
}

/// 下载某语言的 OCR 识别模型（rec 模型 + 字典）。
/// 通过 `ocr-download-progress` 事件上报进度；失败时保留 `.part` 以便断点续传。
#[tauri::command]
pub async fn download_ocr_model(app: tauri::AppHandle, language: String) -> Result<(), String> {
    let lang = find_language(&language)?;
    if lang.tier == "planned" {
        return Err(format!("The '{}' model is planned but not available yet", lang.code));
    }

    let dir = model_dir_for(&lang.code);
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create model dir: {}", e))?;

    if let Some(rec) = &lang.rec_model {
        let dest = dir.join(rec);
        // 幂等：rec 模型已存在且完整时跳过，避免「字典失败后重试」重复下载 rec
        if !dest.is_file() {
            let sha = lang.sha256.clone();
            download_file(&app, &lang.code, &lang.urls, &dest, sha.as_deref()).await?;
        }
    }

    if let Some(dict) = &lang.rec_dict {
        let dest = dir.join(dict);
        if !dest.is_file() {
            download_file(&app, &lang.code, &lang.dict_urls, &dest, None).await?;
        }
    }

    Ok(())
}

/// 删除某语言的 OCR 识别模型（rec 模型 + 字典）。
///
/// 安全护栏：仅允许删除 manifest 中 `deletable = true`（builtin_bundle / downloadable）
/// 的语言，且只删 `models/ocr/<code>/` 目录。内置 wheel 的 en/ch_sim 与规划语言拒绝删除。
#[tauri::command]
pub async fn delete_ocr_model(language: String) -> Result<(), String> {
    let lang = find_language(&language)?;

    if !lang.deletable {
        return Err(format!(
            "The '{}' recognition model is built-in and cannot be deleted",
            lang.code
        ));
    }

    let dir = model_dir_for(&lang.code);
    if !dir.exists() {
        return Ok(()); // 已不存在，幂等
    }

    // 仅删目录内的 rec 模型与字典文件，不越界删其他内容
    if let Some(rec) = &lang.rec_model {
        let p = dir.join(rec);
        if p.exists() {
            fs::remove_file(&p)
                .map_err(|e| format!("Failed to delete rec model: {}", e))?;
        }
    }

    // 清空目录（字典文件等一并删除）
    let _ = fs::remove_dir_all(&dir);

    fs_trace!("OCR model deleted: {}", lang.code);
    Ok(())
}
