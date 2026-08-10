#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
//  Version: 3.0.3
//  License: Apache-2.0 (core) / Proprietary (license verification)
//
//  This file contains the full Rust backend for FrameScout, including:
//  - Tauri IPC commands (scan, search, cluster, smart folders, licensing)
//  - SQLite database management (WAL mode)
//  - In-memory flat vector matrix for O(N·d) brute-force search
//  - ZeroMQ REQ socket for Python inference worker communication
//  - Ed25519 offline license verification (gated behind `pro` feature)
// =========================================================================

use prost::Message;
use std::sync::Mutex;
use std::sync::RwLock;
use walkdir::WalkDir;
use rusqlite::{params, Connection};

use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
  
use std::env;

use std::fs;
use tauri::{AppHandle, Emitter, RunEvent};

// =========================================================================
//  Pro Feature Gate
//  When compiled with `--features pro`, license verification is enabled.
//  Without the feature, the app runs in unlimited mode (for development).
// =========================================================================
#[cfg(feature = "pro")]
mod license {
    use super::*;
    use ed25519_dalek::{Verifier, VerifyingKey, Signature};
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    pub const DEVELOPER_PUBLIC_KEY_HEX: &str = "4e5707fcc7233e50254479f5defb781a6ba0474c0b087809a234bfba9368316c";
    pub const PRODUCT_TAG: &str = "FRAMESCOUT_PRO_V3";

    pub fn verify_license_key(email: &str, license_key: &str) -> Result<(), String> {
        let pub_key_bytes = hex::decode(DEVELOPER_PUBLIC_KEY_HEX)
            .map_err(|e| format!("Failed to decode public key hex: {}", e))?;
        let verifying_key = VerifyingKey::try_from(pub_key_bytes.as_slice())
            .map_err(|e| format!("Failed to parse public key: {}", e))?;

        let decoded_bytes = BASE64.decode(license_key.trim())
            .map_err(|e| format!("Base64 decode failed: {}", e))?;

        let sep_pos = decoded_bytes.windows(2)
            .position(|w| w == b"::")
            .ok_or_else(|| "Missing '::' separator in license key".to_string())?;

        let payload_bytes = &decoded_bytes[..sep_pos];
        let sig_bytes = &decoded_bytes[sep_pos + 2..];

        if sig_bytes.len() != 64 {
            return Err(format!("Invalid signature length: expected 64, got {}", sig_bytes.len()));
        }

        let expected_payload = format!("{}|{}", email.trim().to_lowercase(), PRODUCT_TAG);
        let expected_bytes = expected_payload.as_bytes();
        if payload_bytes != expected_bytes {
            return Err("Payload mismatch: email or product tag incorrect".to_string());
        }

        let signature = Signature::from_slice(sig_bytes)
            .map_err(|e| format!("Invalid signature bytes: {}", e))?;
        verifying_key.verify(payload_bytes, &signature)
            .map_err(|e| format!("Signature verification failed: {}", e))?;

        Ok(())
    }

    pub fn check_local_license() -> (bool, String) {
        let app_data_dir = dirs::data_local_dir().unwrap().join("FrameScout—Offline_AI_Search—Global");
        let lic_path = app_data_dir.join("framescout.lic");

        match fs::read_to_string(lic_path) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                if lines.len() >= 2 {
                    let email = lines[0].trim();
                    let key = lines[1].trim();
                    match verify_license_key(email, key) {
                        Ok(()) => {
                            println!("💎 Pro License Verified for: {}", email);
                            (true, email.to_string())
                        }
                        Err(e) => {
                            println!("⚠️ License verification failed: {}", e);
                            (false, "".to_string())
                        }
                    }
                } else {
                    (false, "".to_string())
                }
            }
            Err(_) => (false, "".to_string()),
        }
    }
}

// =========================================================================
//  Free Trial Guard
//  Limits free users to FREE_TRIAL_LIMIT indexed images.
//  Pro users (or builds without `pro` feature) have no limit.
// =========================================================================
pub const FREE_TRIAL_LIMIT: usize = 100;
// 🌟 New：SigLIP 2.0 uses 768-D embeddings for better semantic understanding.
pub const VECTOR_DIM: usize = 768;

pub struct TrialGuard {
    pub is_pro: bool,
    pub user_email: String,
}

impl TrialGuard {
    pub fn new() -> Self {
        #[cfg(feature = "pro")]
        {
            let (is_pro, user_email) = license::check_local_license();
            Self { is_pro, user_email }
        }
        #[cfg(not(feature = "pro"))]
        {
            // Open source edition: always free tier with 100-image limit
            Self { is_pro: false, user_email: String::new() }
        }
    }

    pub fn check_limit(&self, current_count: usize) -> Result<(), String> {
        if self.is_pro {
            return Ok(());
        }
        if current_count >= FREE_TRIAL_LIMIT {
            return Err(format!(
                "You've hit the free trial cap of {} images. 🚀 Upgrade to Pro and index without limits!",
                FREE_TRIAL_LIMIT
            ));
        }
        Ok(())
    }
}

// =========================================================================
//  FlatVectorMatrix — Cache-friendly brute-force vector store
//  All 768-D embeddings stored contiguously in a single Vec<f32>.
//  Search is O(N·d) dot product — fast enough for N < 50,000.
// =========================================================================
#[derive(Default, Clone)]
pub struct ImageMeta {
    pub path: String,
    pub timestamp: f32,
    pub ocr_text: String,
    pub user_note: String,
    pub index_time: f64,   // 🌟 New
}

#[derive(Default)]
pub struct FlatVectorMatrix {
    pub dim: usize,
    pub flat_vectors: Vec<f32>,
    pub metadata: Vec<ImageMeta>,
}

impl FlatVectorMatrix {
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            flat_vectors: Vec::with_capacity(1000 * dim),
            metadata: Vec::with_capacity(1000),
        }
    }

    pub fn len(&self) -> usize {
        self.metadata.len()
    }

    pub fn is_empty(&self) -> bool {
        self.metadata.is_empty()
    }

    pub fn push(&mut self, path: String, timestamp: f32, vector: Vec<f32>, ocr_text: String, user_note: String, index_time: f64) {
        if vector.len() != self.dim {
            println!("⚠️ Dimension mismatch: expected {}, got {}", self.dim, vector.len());
            return;
        }
        self.flat_vectors.extend_from_slice(&vector);
        self.metadata.push(ImageMeta {
            path,
            timestamp,
            ocr_text,
            user_note,
            index_time,   // 🌟 New
        });
    }

    pub fn contains_path(&self, path: &str) -> bool {
        self.metadata.iter().any(|m| m.path == path)
    }

    /// O(N·d) brute-force cosine similarity search
    pub fn search(&self, query_vec: &[f32], top_k: usize) -> Vec<(usize, f32)> {
        if self.flat_vectors.is_empty() || query_vec.len() != self.dim {
            return Vec::new();
        }

        let mut scores: Vec<(usize, f32)> = self.flat_vectors
            .chunks_exact(self.dim)
            .enumerate()
            .map(|(idx, chunk)| {
                let score: f32 = query_vec.iter().zip(chunk.iter()).map(|(a, b)| a * b).sum();
                (idx, score)
            })
            .collect();

        scores.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    pub fn remove_by_paths(&mut self, paths_to_remove: &[String]) {
        let mut new_flat = Vec::with_capacity(self.flat_vectors.len());
        let mut new_meta = Vec::with_capacity(self.metadata.len());

        for (idx, meta) in self.metadata.drain(..).enumerate() {
            if !paths_to_remove.contains(&meta.path) {
                let start = idx * self.dim;
                let end = start + self.dim;
                new_flat.extend_from_slice(&self.flat_vectors[start..end]);
                new_meta.push(meta);
            }
        }

        self.flat_vectors = new_flat;
        self.metadata = new_meta;
    }
}

// =========================================================================
//  Visual Clustering Result
// =========================================================================
#[derive(serde::Serialize)]
pub struct ClusterGroup {
    pub group_id: usize,
    pub representative_path: String,
    pub member_paths: Vec<String>,
}

// =========================================================================
//  Application State
//  Mutex-wrapped shared resources. Future: replace with RwLock for
//  concurrent read access during scanning.
// =========================================================================
struct AppState {
    db_conn: Mutex<Connection>,
    memory_db: RwLock<FlatVectorMatrix>,
    trial_guard: Mutex<TrialGuard>,
}

// =========================================================================
//  Smart Folder Struct
// =========================================================================
#[derive(serde::Serialize)]
pub struct SmartFolder {
    pub id: i64,
    pub name: String,
    pub query_text: String,
    pub use_vector: bool,
    pub use_ocr: bool,
    pub use_note: bool,
    pub use_filename: bool,
    pub match_count: usize, // 🌟 Dynamic hit count evaluated against backend memory matrix
}

// =========================================================================
//  Progress Payload (sent to frontend via Tauri events)
// =========================================================================
#[derive(Clone, serde::Serialize)]
struct ProgressPayload {
    status: String,
    file_path: String,
    current: usize,
    total: usize,
    new_files: Vec<String>,
}

#[derive(serde::Serialize)]
struct SearchResult {
    path: String,
    timestamp: f32,
    score: f32,
    matched_tags: Vec<String>,
    ocr_text: String,
    user_note: String,
    index_time: f64,  // 🌟 New
}

#[derive(serde::Serialize)]
struct PagedResponse {
    items: Vec<SearchResult>,
    total_count: usize,
}

// =========================================================================
//  ZMQ Request Helper
//  Sends a protobuf-encoded request to the Python worker and awaits reply.
//  Includes automatic reconnection on failure.
// =========================================================================
fn request_vector(
    _state: &tauri::State<AppState>,
    payload: proto::encode_request::Payload,
    timeout_ms: i32,
) -> Result<Vec<(String, f32, Vec<f32>, String, f64)>, String> {
    let context = zmq::Context::new();
    let socket = context.socket(zmq::REQ).map_err(|e| e.to_string())?;
    socket.set_rcvtimeo(timeout_ms).map_err(|e| e.to_string())?;
    socket.connect("tcp://127.0.0.1:5555").map_err(|e| e.to_string())?;

    let req = proto::EncodeRequest {
        task_id: "TASK_GLOBAL".to_string(),
        payload: Some(payload),
        single_file_ocr_config: None,
    };
    let mut buf = Vec::new();
    req.encode(&mut buf).unwrap();

    socket.send(buf, 0).map_err(|e| e.to_string())?;
    let reply_raw = socket.recv_bytes(0).map_err(|e| e.to_string())?;

    let res = proto::EncodeResponse::decode(&reply_raw[..]).map_err(|e| e.to_string())?;
    match res.result {
        Some(proto::encode_response::Result::Success(s)) => Ok(s
            .frames
            .into_iter()
            .map(|f| (f.file_path, f.timestamp, f.vector, f.ocr_text, f.index_time))
            .collect()),
        Some(proto::encode_response::Result::Error(e)) => Err(e.message),
        None => Err("Unknown response".to_string()),
    }
}

// =========================================================================
//  Database Initialization & Memory Matrix Loading
// =========================================================================
fn init_db_and_load_memory() -> (Connection, FlatVectorMatrix) {
    println!("💾 Connecting to local SQLite Hybrid Matrix...");
    let app_data_dir = dirs::data_local_dir().unwrap().join("FrameScout—Offline_AI_Search—Global");
    if let Err(e) = fs::create_dir_all(&app_data_dir) {
        println!("⚠️ Warning: Failed to create AppData directory: {}", e);
    }

    let db_path = app_data_dir.join("framescout—offline_ai_search—global.db");
    let conn = Connection::open(&db_path).expect("Failed to open database");

    // WAL mode allows concurrent reads during writes
    let _ = conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;");

    // Create frame_vectors table
    if let Err(e) = conn.execute(
        "CREATE TABLE IF NOT EXISTS frame_vectors (
            path TEXT PRIMARY KEY,
            timestamp REAL NOT NULL,
            vector_json TEXT NOT NULL,
            ocr_text TEXT DEFAULT '',
            user_note TEXT DEFAULT '',
            index_time REAL DEFAULT 0.0   -- 🌟 New column
        )",
        [],
    ) {
        println!("⚠️ Warning: Failed to create frame_vectors table: {}", e);
    }

    // Create smart_folders table
    if let Err(e) = conn.execute(
        "CREATE TABLE IF NOT EXISTS smart_folders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            query_text TEXT NOT NULL,
            use_vector INTEGER DEFAULT 1,
            use_ocr INTEGER DEFAULT 1,
            use_note INTEGER DEFAULT 1,
            use_filename INTEGER DEFAULT 1
        )",
        [],
    ) {
        println!("⚠️ Warning: Failed to create smart_folders table: {}", e);
    }

    // 🌟 Original 768 -D embeddings are now the standard. Old 512-D vectors will be auto-cleaned.
    let mut memory_matrix = FlatVectorMatrix::new(VECTOR_DIM);
    let mut stale_paths = Vec::new();

    // Load all vectors into memory matrix
    if let Ok(mut stmt) = conn.prepare(
        "SELECT path, timestamp, vector_json, ocr_text, user_note, index_time FROM frame_vectors
         ORDER BY index_time DESC",
    ) {
        let rows = stmt.query_map([], |row| {
            let path: String = row.get(0)?;
            let ts: f64 = row.get(1)?;
            let json_str: String = row.get(2)?;
            let ocr_text: String = row.get(3)?;
            let user_note: String = row.get(4).unwrap_or_default();
            let index_time: f64 = row.get(5)?;
            let vector: Vec<f32> = serde_json::from_str(&json_str).unwrap_or_default();
            Ok((path, ts as f32, vector, ocr_text, user_note, index_time))
        });

        if let Ok(rows) = rows {
            for row in rows {
                if let Ok((path, ts, vector, ocr_text, user_note, index_time)) = row {
                    // 🌟 Check if the dimension matches the current VECTOR_DIM (768)
                    if vector.len() == VECTOR_DIM {
                        memory_matrix.push(path, ts, vector, ocr_text, user_note, index_time);
                    } else {
                        // Mark obsolete vectors (e.g., old 512-D data), preparing for automatic cleanup
                        println!("⚠️ Found obsolete vector (dim: {}) for path: {}. Marking for clean.", vector.len(), path);
                        stale_paths.push(path);
                    }
                }
            }
        }
    }

    // 🌟 Clean up obsolete dimension data to prevent residual redundant records in the database
    if !stale_paths.is_empty() {
        println!("🧹 Cleaning up {} obsolete database records...", stale_paths.len());
        for path in stale_paths {
            let _ = conn.execute("DELETE FROM frame_vectors WHERE path = ?1", params![path]);
        }
    }

    println!(
        "✅ Memory matrix loaded! Holding {} spatio-temporal slices (Dim: {}).",
        memory_matrix.len(),
        VECTOR_DIM
    );
    (conn, memory_matrix)
}

// =========================================================================
// Helper: Process explicit list of file paths (Used by scan_folder & drag-and-drop)
// =========================================================================
async fn process_file_paths_internal(
    app: &AppHandle,
    state: &tauri::State<'_, AppState>,
    pending_files: Vec<String>,
    enable_ocr: bool,
    ocr_languages: Vec<String>,
) -> Result<usize, String> {

    let total_to_process = pending_files.len();

    // Early exit if no new files to process
    if total_to_process == 0 {
        let _ = app.emit(
            "scan-progress",
            ProgressPayload {
                status: "✅ Done".to_string(),
                file_path: "No new files to process".to_string(),
                current: 0,
                total: 0,
                new_files: vec![],
            },
        );
        return Ok(0);
    }

    if enable_ocr && ocr_languages.is_empty() {
        return Err("OCR enabled but no languages specified".to_string());
    }

    let mut consecutive_failures = 0;
    const MAX_CONSECUTIVE_FAILURES: usize = 5;

    let mut added_count = 0;
    let mut current_idx = 0;
    let batch_size = 4;

    for chunk in pending_files.chunks(batch_size) {

        // Check trial limit
        {
            let memory = state.memory_db.read().map_err(|e| e.to_string())?;
            if let Err(err_msg) = state.trial_guard.lock().unwrap().check_limit(memory.len()) {
                let _detailed_msg = format!(
                    "{} (currently indexed: {}, limit: {})",
                    err_msg, memory.len(), FREE_TRIAL_LIMIT
                );
                let _ = app.emit(
                    "scan-progress",
                    ProgressPayload {
                        status: "🛑 Trial Limit Reached".to_string(),
                        file_path: err_msg.clone(),
                        current: current_idx,
                        total: total_to_process,
                        new_files: vec![],
                    },
                );
                return Err(err_msg);
            }
        }
        current_idx += chunk.len();
        
        let batch_payload = proto::BatchPaths {
            file_paths: chunk.to_vec(),
            ocr_config: Some(proto::OcrConfig {
                enable_ocr,
                languages: ocr_languages.clone(),
            }),
        };
        let payload = proto::encode_request::Payload::Batch(batch_payload);

        match request_vector(state, payload, -1) {
            Ok(frames) => {

                consecutive_failures = 0;

                let mut db = state.db_conn.lock().unwrap();
                let mut memory = state.memory_db.write().map_err(|e| e.to_string())?;

                let tx_result = db.transaction();
                let mut batch_added_files = Vec::new();

                if let Ok(tx) = tx_result {
                    for (file_path, timestamp, vec, ocr_text, index_time) in frames {
                        let vec_json = serde_json::to_string(&vec).unwrap_or_else(|_| "[]".to_string());
                        if tx.execute(
                            "INSERT INTO frame_vectors (path, timestamp, vector_json, ocr_text, user_note, index_time) 
                             VALUES (?1, ?2, ?3, ?4, '', ?5)
                             ON CONFLICT(path) DO UPDATE SET 
                             ocr_text = excluded.ocr_text, 
                             index_time = excluded.index_time",
                            params![file_path.clone(), timestamp, vec_json, ocr_text.clone(), index_time],
                        ).is_ok() {
                            memory.push(file_path.clone(), timestamp, vec, ocr_text, "".to_string(), index_time);
                            batch_added_files.push(file_path);
                            added_count += 1;
                        }
                    }
                    let _ = tx.commit();
                }

                let _ = app.emit(
                    "scan-progress",
                    ProgressPayload {
                        status: format!(
                            "🚀 Batch Processing ({}/{}), {} per group...",
                            current_idx,
                            total_to_process,
                            chunk.len()
                        ),
                        file_path: chunk.last().cloned().unwrap_or_default(),
                        current: current_idx,
                        total: total_to_process,
                        new_files: batch_added_files,
                    },
                );
            }
            Err(e) => {
                consecutive_failures += 1;
                let _ = app.emit(
                    "scan-progress",
                    ProgressPayload {
                        status: format!("❌ Batch Failed: {}", e),
                        file_path: chunk.last().cloned().unwrap_or_default(),
                        current: current_idx,
                        total: total_to_process,
                        new_files: vec![],
                    },
                );
                if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                    let _ = app.emit(
                        "scan-progress",
                        ProgressPayload {
                            status: "🛑 Aborted: Too many consecutive failures".to_string(),
                            file_path: chunk.last().cloned().unwrap_or_default(),
                            current: current_idx,
                            total: total_to_process,
                            new_files: vec![],
                        },
                    );
                    return Err(format!(
                        "Aborted after {} consecutive failures: {}",
                        consecutive_failures, e
                    ));
                }
            }
        }
    }
    let _ = app.emit(
        "scan-progress",
        ProgressPayload {
            status: "✅ Done".to_string(),
            file_path: "Queue Finished".to_string(),
            current: total_to_process,
            total: total_to_process,
            new_files: vec![],
        },
    );

    Ok(added_count)
}

// =========================================================================
//  Tauri Commands
// =========================================================================

/// Ping the Python inference engine to check if it's alive
#[tauri::command]
async fn ping_engine(_state: tauri::State<'_, AppState>) -> Result<String, String> {
    match request_vector(
        &_state,
        proto::encode_request::Payload::Text("PING_ENGINE".to_string()),
        90000,
    ) {
        Ok(_) => Ok("READY".to_string()),
        Err(e) => Err(e),
    }
}

/// Scan a folder and index all images/videos
/// Updated Scan Folder Command with explicit OCR Toggles and Language configs
#[tauri::command]
async fn scan_folder(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    folder_path: String,
    scan_mode: String,
    enable_ocr: bool,
    ocr_languages: Vec<String>,
) -> Result<usize, String> {

    // Check trial limit
    {
        let memory = state.memory_db.read().map_err(|e| e.to_string())?;
        state.trial_guard.lock().unwrap().check_limit(memory.len())?;
    }

    let _ = app.emit(
        "scan-progress",
        ProgressPayload {
            status: "🔍 Rapid Pre-scanning...".to_string(),
            file_path: "".to_string(),
            current: 0,
            total: 0,
            new_files: vec![],
        },
    );

    let mut pending_files = Vec::new();

    for entry in WalkDir::new(&folder_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            let is_image = ext == "jpg" || ext == "jpeg" || ext == "png" || ext == "webp";
            let is_video = ext == "mp4" || ext == "mov" || ext == "avi" || ext == "mkv" || ext == "webm" || ext == "flv";
            let should_process = match scan_mode.as_str() {
                "image" => is_image,
                "video" => is_video,
                _ => is_image || is_video,
            };

            if should_process {
                let path_str = path.to_string_lossy().to_string();
                let exists = { state.memory_db.read().map_err(|e| e.to_string())?.contains_path(&path_str) };
                if !exists {
                    pending_files.push(path_str);
                }
            }
        }
    }

    if pending_files.is_empty() {
        let _ = app.emit(
            "scan-progress",
            ProgressPayload {
                status: "✅ Done".to_string(),
                file_path: "No new files found".to_string(),
                current: 0,
                total: 0,
                new_files: vec![],
            },
        );
        return Ok(0);
    }

    process_file_paths_internal(&app, &state, pending_files, enable_ocr, ocr_languages).await
}

/// 🌟 Target Indexing: Direct Drag-and-Drop or Specific Batch File Indexing
#[tauri::command]
async fn index_files(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    file_paths: Vec<String>,
    enable_ocr: bool,
    ocr_languages: Vec<String>,
) -> Result<usize, String> {

    // Check trial limit
    {
        let memory = state.memory_db.read().map_err(|e| e.to_string())?;
        state.trial_guard.lock().unwrap().check_limit(memory.len())?;
    }
    let _ = app.emit(
        "scan-progress",
        ProgressPayload {
            status: format!("📁 Indexing {} files...", file_paths.len()),
            file_path: "".to_string(),
            current: 0,
            total: file_paths.len(),
            new_files: vec![],
        },
    );

    let mut pending_files = Vec::new();
    let total = file_paths.len();

    {
        let memory = state.memory_db.read().map_err(|e| e.to_string())?;
        for path_str in &file_paths {
            if !memory.contains_path(&path_str) {
                pending_files.push(path_str.clone());
            }
        }
    }

    if pending_files.is_empty() {
        let _ = app.emit(
            "scan-progress",
            ProgressPayload {
                status: "✅ Done".to_string(),
                file_path: "All files already indexed".to_string(),
                current: total,
                total: total,
                new_files: vec![],
            },
        );
        return Ok(0);
    }

    process_file_paths_internal(&app, &state, pending_files, enable_ocr, ocr_languages).await
}

/// 🌟 On-Demand OCR: Run or update OCR specifically for selected files with target languages
#[tauri::command]
async fn run_ocr_for_selected_files(
    state: tauri::State<'_, AppState>,
    file_paths: Vec<String>,
    languages: Vec<String>,
) -> Result<usize, String> {
    if file_paths.is_empty() {
        return Ok(0);
    }

    let ocr_task_payload = proto::SingleOcrTask {
        file_paths: file_paths.clone(),
        languages: languages.clone(),
    };
    let payload = proto::encode_request::Payload::OcrTask(ocr_task_payload);

    let frames = request_vector(&state, payload, -1)?;
    let db = state.db_conn.lock().unwrap();
    let mut memory = state.memory_db.write().map_err(|e| e.to_string())?;

    let mut updated_count = 0;
    for (file_path, _, _, extracted_text, _) in frames {
        if db.execute(
            "UPDATE frame_vectors SET ocr_text = ?1 WHERE path = ?2",
            params![extracted_text.clone(), file_path.clone()],
        ).is_ok() {
            // Update in-memory metadata
            for meta in memory.metadata.iter_mut() {
                if meta.path == file_path {
                    meta.ocr_text = extracted_text.clone();
                }
            }
            updated_count += 1;
        }
    }

    Ok(updated_count)
}

/// Visual clustering — O(N²) brute-force pairwise similarity
#[tauri::command]
async fn cluster_similar_images(
    state: tauri::State<'_, AppState>,
    threshold: f32,
) -> Result<Vec<ClusterGroup>, String> {
    let memory = state.memory_db.read().map_err(|e| e.to_string())?;
    if memory.is_empty() {
        return Ok(Vec::new());
    }

    let dim = memory.dim;
    let num_items = memory.len();
    let mut visited = vec![false; num_items];
    let mut groups = Vec::new();

    for i in 0..num_items {
        if visited[i] {
            continue;
        }
        visited[i] = true;

        let vec_i = &memory.flat_vectors[i * dim..(i + 1) * dim];
        let mut members = vec![memory.metadata[i].path.clone()];

        for j in (i + 1)..num_items {
            if visited[j] {
                continue;
            }
            let vec_j = &memory.flat_vectors[j * dim..(j + 1) * dim];
            let similarity: f32 = vec_i.iter().zip(vec_j.iter()).map(|(a, b)| a * b).sum();

            if similarity >= threshold {
                visited[j] = true;
                members.push(memory.metadata[j].path.clone());
            }
        }

        if members.len() > 1 {
            groups.push(ClusterGroup {
                group_id: groups.len() + 1,
                representative_path: members[0].clone(),
                member_paths: members,
            });
        }
    }

    Ok(groups)
}

/// Update personal note for an image
#[tauri::command]
async fn update_note(
    state: tauri::State<'_, AppState>,
    path: String,
    note: String,
) -> Result<(), String> {
    let db = state.db_conn.lock().unwrap();
    db.execute(
        "UPDATE frame_vectors SET user_note = ?1 WHERE path = ?2",
        params![note.clone(), path.clone()],
    )
    .map_err(|e| e.to_string())?;

    let mut memory = state.memory_db.write().map_err(|e| e.to_string())?;
    for meta in memory.metadata.iter_mut() {
        if meta.path == path {
            meta.user_note = note.clone();
        }
    }
    Ok(())
}

/// Remove records for files that no longer exist on disk
#[tauri::command]
async fn clean_ghosts(state: tauri::State<'_, AppState>) -> Result<usize, String> {
    let db = state.db_conn.lock().unwrap();
    let mut memory = state.memory_db.write().map_err(|e| e.to_string())?;
    let mut to_remove = Vec::new();

    for meta in &memory.metadata {
        if !std::path::Path::new(&meta.path).exists() {
            to_remove.push(meta.path.clone());
        }
    }

    for p in &to_remove {
        db.execute("DELETE FROM frame_vectors WHERE path = ?1", params![p])
            .map_err(|e| format!("Failed to delete ghost record: {}", e))?;
    }
    memory.remove_by_paths(&to_remove);
    Ok(to_remove.len())
}

/// Multi-modal search: vector + OCR + notes + filename
#[tauri::command]
async fn search_images(
    state: tauri::State<'_, AppState>,
    text: String,
    page: usize,
    limit: usize,
    use_vector: bool,
    use_ocr: bool,
    use_note: bool,
    use_filename: bool,
) -> Result<PagedResponse, String> {
    {
        let memory = state.memory_db.read().map_err(|e| e.to_string())?;
        if memory.is_empty() {
            return Err("Memory Matrix is empty! Please scan a folder first.".to_string());
        }
    }

    let mut text_vec = Vec::new();
    if use_vector {
        let text_frames =
            request_vector(&state, proto::encode_request::Payload::Text(text.clone()), -1)?;
        if !text_frames.is_empty() {
            text_vec = text_frames[0].2.clone();
        }
    }

    let memory = state.memory_db.read().map_err(|e| e.to_string())?;
    if memory.is_empty() {
        return Err("Memory Matrix is empty!".to_string());
    }

    let vector_scores = if use_vector && !text_vec.is_empty() {
        memory.search(&text_vec, memory.len())
    } else {
        Vec::new()
    };

    let score_map: std::collections::HashMap<usize, f32> = vector_scores.into_iter().collect();
    let lower_search = text.to_lowercase();
    let mut results: Vec<SearchResult> = Vec::new();

    for (idx, meta) in memory.metadata.iter().enumerate() {
        let mut score: f32 = *score_map.get(&idx).unwrap_or(&0.0);
        let mut matched_tags = Vec::new();

        if score > 0.001 {
            matched_tags.push("💡 Semantic".to_string());
        } else {
            score = 0.0;
        }

        if use_ocr && !meta.ocr_text.is_empty() && meta.ocr_text.to_lowercase().contains(&lower_search)
        {
            score += 2.0;
            matched_tags.push("🔍 OCR".to_string());
        }
        if use_note
            && !meta.user_note.is_empty()
            && meta.user_note.to_lowercase().contains(&lower_search)
        {
            score += 2.5;
            matched_tags.push("📝 Note".to_string());
        }
        if use_filename && meta.path.to_lowercase().contains(&lower_search) {
            score += 3.0;
            matched_tags.push("📁 Filename".to_string());
        }

        if score > 0.001 || !matched_tags.is_empty() {
            results.push(SearchResult {
                path: meta.path.clone(),
                timestamp: meta.timestamp,
                score,
                matched_tags,
                ocr_text: meta.ocr_text.clone(),
                user_note: meta.user_note.clone(),
                index_time: meta.index_time,
            });
        }
    }

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Deduplicate by normalized path
    let mut unique_results = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();
    for res in results {
        let normalized = res.path.to_lowercase().replace("\\", "/");
        if !seen_paths.contains(&normalized) {
            seen_paths.insert(normalized);
            unique_results.push(res);
        }
    }

    let total_count = unique_results.len();
    let start_index = (page.saturating_sub(1)) * limit;
    let items = unique_results.into_iter().skip(start_index).take(limit).collect();
    Ok(PagedResponse { items, total_count })
}

/// Image-to-image search
#[tauri::command]
async fn search_by_image(
    state: tauri::State<'_, AppState>,
    image_path: String,
    page: usize,
    limit: usize,
) -> Result<PagedResponse, String> {
    {
        let memory = state.memory_db.read().map_err(|e| e.to_string())?;
        if memory.is_empty() {
            return Err("Memory Matrix is empty! Please scan a folder first.".to_string());
        }
    }

    let img_frames =
        request_vector(&state, proto::encode_request::Payload::FilePath(image_path), -1)?;
    if img_frames.is_empty() {
        return Err("Failed to extract image vectors".to_string());
    }
    let search_vec = &img_frames[0].2;

    let memory = state.memory_db.read().map_err(|e| e.to_string())?;
    if memory.is_empty() {
        return Err("Memory Matrix is empty!".to_string());
    }

    let vector_scores = memory.search(search_vec, memory.len());

    let mut results: Vec<SearchResult> = vector_scores
        .into_iter()
        .filter_map(|(idx, score)| {
            if score > 0.001 {
                let meta = &memory.metadata[idx];
                Some(SearchResult {
                    path: meta.path.clone(),
                    timestamp: meta.timestamp,
                    score,
                    matched_tags: vec!["🖼️ Visual".to_string()],
                    ocr_text: meta.ocr_text.clone(),
                    user_note: meta.user_note.clone(),
                    index_time: meta.index_time,
                })
            } else {
                None
            }
        })
        .collect();

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut unique_results = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();
    for res in results {
        let normalized = res.path.to_lowercase().replace("\\", "/");
        if !seen_paths.contains(&normalized) {
            seen_paths.insert(normalized);
            unique_results.push(res);
        }
    }

    let total_count = unique_results.len();
    let start_index = (page.saturating_sub(1)) * limit;
    let items = unique_results.into_iter().skip(start_index).take(limit).collect();
    Ok(PagedResponse { items, total_count })
}

/// Save a smart folder (saved search)
#[tauri::command]
async fn save_smart_folder(
    state: tauri::State<'_, AppState>,
    name: String,
    query_text: String,
    use_vector: bool,
    use_ocr: bool,
    use_note: bool,
    use_filename: bool,
) -> Result<i64, String> {
    let db = state.db_conn.lock().unwrap();
    db.execute(
        "INSERT INTO smart_folders (name, query_text, use_vector, use_ocr, use_note, use_filename) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![name, query_text, use_vector as i32, use_ocr as i32, use_note as i32, use_filename as i32],
    ).map_err(|e| e.to_string())?;
    Ok(db.last_insert_rowid())
}

/// List all smart folders and calculate real-time match counts on the backend
#[tauri::command]
async fn get_smart_folders(state: tauri::State<'_, AppState>) -> Result<Vec<SmartFolder>, String> {
    let db = state.db_conn.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT id, name, query_text, use_vector, use_ocr, use_note, use_filename FROM smart_folders")
        .map_err(|e| e.to_string())?;

    let memory = state.memory_db.read().map_err(|e| e.to_string())?;
    
    let rows = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let query_text: String = row.get(2)?;
            let v: i32 = row.get(3)?;
            let o: i32 = row.get(4)?;
            let n: i32 = row.get(5)?;
            let f: i32 = row.get(6)?;
            Ok((id, name, query_text, v == 1, o == 1, n == 1, f == 1))
        })
        .map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    let lower_search_list: Vec<(i64, String, String, bool, bool, bool, bool)> = rows.filter_map(|r| r.ok()).collect();

    for (id, name, query_text, use_vector, use_ocr, use_note, use_filename) in lower_search_list {
        // Calculate dynamic match count directly against in-memory matrix
        let mut count = 0;
        let lower_query = query_text.to_lowercase();

        if !lower_query.is_empty() {
            for meta in &memory.metadata {
                let mut matched = false;
                if use_ocr && !meta.ocr_text.is_empty() && meta.ocr_text.to_lowercase().contains(&lower_query) {
                    matched = true;
                }
                if use_note && !meta.user_note.is_empty() && meta.user_note.to_lowercase().contains(&lower_query) {
                    matched = true;
                }
                if use_filename && meta.path.to_lowercase().contains(&lower_query) {
                    matched = true;
                }
                if matched {
                    count += 1;
                }
            }
        }

        list.push(SmartFolder {
            id,
            name,
            query_text,
            use_vector,
            use_ocr,
            use_note,
            use_filename,
            match_count: count,
        });
    }

    Ok(list)
}

/// 🌟 Execute a Backend Smart Folder directly by ID
#[tauri::command]
async fn execute_smart_folder(
    state: tauri::State<'_, AppState>,
    id: i64,
    page: usize,
    limit: usize,
) -> Result<PagedResponse, String> {
    let (query_text, use_vector, use_ocr, use_note, use_filename) = {
        let db = state.db_conn.lock().unwrap();
        db.query_row(
            "SELECT query_text, use_vector, use_ocr, use_note, use_filename FROM smart_folders WHERE id = ?1",
            params![id],
            |row| {
                let q: String = row.get(0)?;
                let v: i32 = row.get(1)?;
                let o: i32 = row.get(2)?;
                let n: i32 = row.get(3)?;
                let f: i32 = row.get(4)?;
                Ok((q, v == 1, o == 1, n == 1, f == 1))
            },
        ).map_err(|e| format!("Smart folder not found: {}", e))?
    };

    // Re-use core backend multi-modal search engine
    search_images(
        state,
        query_text,
        page,
        limit,
        use_vector,
        use_ocr,
        use_note,
        use_filename,
    ).await
}

/// Delete a smart folder
#[tauri::command]
async fn delete_smart_folder(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db_conn.lock().unwrap();
    db.execute("DELETE FROM smart_folders WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Serialize)]
struct PaginatedResult {
    items: Vec<SearchResult>,
    total_count: usize,
}

#[tauri::command]
fn list_all_files(
    state: tauri::State<'_, AppState>,
    page: u32,
    limit: u32,
) -> Result<PaginatedResult, String> {
    let db = state.db_conn.lock().map_err(|e| e.to_string())?;
    let offset = (page.saturating_sub(1)) * limit;

    // Get the total count
    let total_count: i64 = db
        .query_row("SELECT COUNT(*) FROM frame_vectors", [], |row| row.get(0))
        .map_err(|e| format!("DB count error: {}", e))?;
    let total_count = total_count as usize;

    // Fetch paginated results
    let mut stmt = db
        .prepare(
            "SELECT path, timestamp, vector_json, ocr_text, user_note, index_time
             FROM frame_vectors
             ORDER BY index_time DESC
             LIMIT ?1 OFFSET ?2",
        )
        .map_err(|e| format!("Prepare error: {}", e))?;

    let items = stmt
        .query_map(params![limit, offset], |row| {
            let path: String = row.get(0)?;
            let timestamp: f64 = row.get(1)?;
            let _vector_json: String = row.get(2)?; // Not used in this context, but retrieved for completeness
            let ocr_text: String = row.get(3)?;
            let user_note: String = row.get(4).unwrap_or_default();
            let index_time: f64 = row.get(5).unwrap_or(0.0);
            Ok(SearchResult {
                path,
                timestamp: timestamp as f32,
                score: 2.0, // Placeholder score for listing, not used in this context
                matched_tags: vec!["📂 All Files".to_string()],
                ocr_text,
                user_note,
                index_time,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(PaginatedResult { items, total_count })
}

#[tauri::command]
fn get_all_files(state: tauri::State<'_, AppState>) -> Result<Vec<SearchResult>, String> {
    let db = state.db_conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT path, timestamp, vector_json, ocr_text, user_note, index_time
             FROM frame_vectors
             ORDER BY index_time DESC"
        )
        .map_err(|e| format!("Prepare error: {}", e))?;

    let items = stmt
        .query_map([], |row| {
            let path: String = row.get(0)?;
            let timestamp: f64 = row.get(1)?;
            let _vector_json: String = row.get(2)?;
            let ocr_text: String = row.get(3)?;
            let user_note: String = row.get(4).unwrap_or_default();
            let index_time: f64 = row.get(5).unwrap_or(0.0);
            Ok(SearchResult {
                path,
                timestamp: timestamp as f32,
                score: 2.0,
                matched_tags: vec!["📂 All Files".to_string()],
                ocr_text,
                user_note,
                index_time,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(items)
}

fn wait_for_engine_and_notify(app_handle: tauri::AppHandle) {
    let context = zmq::Context::new();
    let mut retries = 0;
    const MAX_RETRIES: u32 = 50;

    loop {
        // REQ socket must receive a reply before sending again
        let check_socket = context.socket(zmq::REQ).unwrap();
        check_socket.connect("tcp://127.0.0.1:5555").unwrap();
        check_socket.set_rcvtimeo(3000).unwrap();

        let ping_req = proto::EncodeRequest {
            task_id: "PING_INIT".to_string(),
            payload: Some(proto::encode_request::Payload::Text("PING_ENGINE".to_string())),
            single_file_ocr_config: None,
        };
        let mut buf = Vec::new();
        ping_req.encode(&mut buf).unwrap();

        let success = match check_socket.send(buf, 0) {
            Ok(()) => match check_socket.recv_bytes(0) {
                Ok(reply) => {
                    if let Ok(resp) = proto::EncodeResponse::decode(&reply[..]) {
                        resp.result.is_some()
                    } else {
                        false
                    }
                }
                Err(_) => false,
            },
            Err(_) => false,
        };

        drop(check_socket);

        if success {
            let _ = app_handle.emit("engine-status", serde_json::json!({
                "status": "ready",
                "message": "AI Worker is ready!"
            }));
            println!("✅ AI Worker is ready!");
            break;
        }

        retries += 1;
        let _ = app_handle.emit("engine-status", serde_json::json!({
            "status": "connecting",
            "retry": retries,
            "max_retries": MAX_RETRIES,
            "message": format!("Connecting... attempt {}/{}", retries, MAX_RETRIES)
        }));

        if retries >= MAX_RETRIES {
            let _ = app_handle.emit("engine-status", serde_json::json!({
                "status": "error",
                "message": "AI Worker failed to become ready. Please check models and restart."
            }));
            panic!("AI Worker failed to become ready within {} seconds.", MAX_RETRIES * 3);
        }

        println!("⏳ Retrying in 3 seconds... (attempt {}/{})", retries, MAX_RETRIES);
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
}

// =========================================================================
//  Main Entry Point
// =========================================================================
mod proto {
    include!(concat!(env!("OUT_DIR"), "/framescout.rs"));
}

fn main() {
    // Kill any stale AI worker processes
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "ai_worker.exe", "/T"])
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        // On Unix-like systems, try to kill existing ai_worker process
        let _ = Command::new("pkill")
            .arg("-f")
            .arg("ai_worker")
            .output();
        // If pkill fails, ignore (process may not exist)
    }
    // Locate the Python inference worker executable
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

    let mut cmd = Command::new(&ai_path);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    let mut ai_process = cmd.spawn().expect("❌ Failed to spawn");

    // Initialize database and load memory matrix
    let (db_conn, memory_db) = init_db_and_load_memory();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            // zmq_socket: Mutex::new(socket),
            db_conn: Mutex::new(db_conn),
            memory_db: RwLock::new(memory_db),
            trial_guard: Mutex::new(TrialGuard::new()),
        })
        .setup(|app| {
            let app_handle = app.handle().clone();
            // Start background thread to wait for engine readiness.
            std::thread::spawn(move || {
                wait_for_engine_and_notify(app_handle);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_folder,
            index_files,                  // 🌟 Registered: Drag-and-drop / target files indexing
            run_ocr_for_selected_files,   // 🌟 Registered: On-demand OCR for specific selected files
            search_images,
            ping_engine,
            update_note,
            clean_ghosts,
            search_by_image,
            cluster_similar_images,
            save_smart_folder,
            get_smart_folders,
            execute_smart_folder,        // 🌟 Registered: Direct backend Smart Folder execution
            delete_smart_folder,
            activate_pro_license,
            get_license_status,
            list_all_files,
            get_all_files,
        ])
        .build(tauri::generate_context!())
        .expect("Tauri Build Fail");

    app.run(move |_, event| {
        if let RunEvent::Exit = event {
            let _ = ai_process.kill();
        }
    });
}
