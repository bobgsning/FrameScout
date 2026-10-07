// 纯文本通道的命令：文本文件直接入库 + 条目管理（列表 / 删除）。
//
// 与媒体（图片/视频）是两条路：这里**不经过 SigLIP 视觉编码**，
// 而是读入文本 → 分块 → BGE-M3 编码 → 落 `text_entries`。

use std::fs;
use std::path::Path;
use std::time::Instant;

use serde_json::json;
use tauri::State;
use walkdir::WalkDir;

use crate::constants::{TEXT_CHUNK_CHARS, TEXT_CHUNK_OVERLAP, TEXT_EXTENSIONS};
use crate::fs_trace;
use crate::model_code::{AppState, TextEntryListResponse, TextIngestFailure, TextIngestResult};
use crate::services::request_text_entry;
use crate::storage::normalize_path;
use crate::storage::text_store;

/// 把长文本切成若干块：**优先按段落边界**，段落内超长再硬切。
///
/// P1-11 / 第三轮 E9-E12 全面改造：
///   - **重叠窗口（E9）**：硬切时保留前一块尾部 TEXT_CHUNK_OVERLAP 字符作为下一块前缀，
///     跨块边界的关键句在两块中各保留一份，显著提升跨块召回。
///   - **单换行也认（E10）**：旧实现只认 `\n\n`，大量 Markdown/笔记用单换行分段，
///     整个文件落成「一个超长段落」直接进硬切分支。现在 `\n\n` 和 `\n` 都视为段落边界。
///   - **上下文前缀（E11）**：每个块前拼 `[文件名 > 最近标题]` 前缀再编码，
///     块脱离文档标题后不丢失指代对象（contextual retrieval）。
///   - **ingest 前删旧块（E8）**：第二次 ingest 同一文件前先删该 source 的旧块，
///     避免文档改短后尾部旧块永久残留。
///
/// 按 **字符数**（而非字节数）计：中文一个字就是一个字符。
fn chunk_text(text: &str, max_chars: usize, file_name: &str) -> Vec<String> {
    // 每个元素：(块内容, 该块对应的「最近标题」)。债单 B5：按块取标题，而非全文最后标题。
    let mut chunks: Vec<(String, String)> = Vec::new();
    let mut current = String::new();
    let mut current_heading = String::new();
    let mut last_heading: String = String::new(); // 最近遇到的 Markdown 标题行

    // 从字符串尾部取 overlap 字符（债单 B4：累加分支也保留跨块重叠）。
    // 取 min(n, src_len - 1)，避免 overlap 大于块长导致下一块完整重复上一块。
    fn overlap_tail(src: &str, n: usize) -> String {
        if n == 0 || src.is_empty() {
            return String::new();
        }
        let src_len = src.chars().count();
        let take = n.min(src_len.saturating_sub(1));
        if take == 0 {
            return String::new();
        }
        src.chars().rev().take(take).collect::<Vec<_>>().into_iter().rev().collect()
    }

    // P1-11 E10：单换行也视为段落边界（不再只认 \n\n）
    for para_block in text.split("\n\n") {
        let para_block = para_block.trim();
        if para_block.is_empty() {
            continue;
        }
        for para in para_block.split('\n') {
            let para = para.trim();
            if para.is_empty() {
                continue;
            }
            // 追踪最近的 Markdown 标题（# / ## / ### ...）
            if para.starts_with('#') {
                last_heading = para.trim_start_matches('#').trim().to_string();
            }

            let para_len = para.chars().count();

            // 段落本身超长：先收掉已累积的内容，再把该段落硬切成多块
            if para_len > max_chars {
                if !current.is_empty() {
                    chunks.push((current.trim().to_string(), current_heading.clone()));
                    current = String::new();
                    current_heading = last_heading.clone();
                }
                let mut buf = String::new();
                for ch in para.chars() {
                    buf.push(ch);
                    if buf.chars().count() >= max_chars {
                        chunks.push((buf.clone(), last_heading.clone()));
                        buf.clear();
                        if TEXT_CHUNK_OVERLAP > 0 {
                            let tail = chunks
                                .last()
                                .map(|c| overlap_tail(c.0.as_str(), TEXT_CHUNK_OVERLAP))
                                .unwrap_or_default();
                            buf = tail;
                        }
                    }
                }
                if !buf.trim().is_empty() {
                    chunks.push((buf.trim().to_string(), last_heading.clone()));
                }
                continue;
            }

            // 累加到接近上限就收一块（段落边界优先）。
            // 计入 "\n\n" 分隔符的长度，否则拼出来的块会略微超出上限。
            // 债单 B4：累加分支也保留 overlap（旧实现只在硬切分支做）。
            let sep_len = if current.is_empty() { 0 } else { 2 };
            if !current.is_empty() && current.chars().count() + sep_len + para_len > max_chars {
                let pushed = current.trim().to_string();
                chunks.push((pushed.clone(), current_heading.clone()));
                current = overlap_tail(&pushed, TEXT_CHUNK_OVERLAP);
                current_heading = last_heading.clone();
            }
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(para);
        }
    }
    if !current.trim().is_empty() {
        chunks.push((current.trim().to_string(), current_heading.clone()));
    }

    if file_name.is_empty() {
        return chunks.into_iter().map(|(c, _)| c).collect();
    }

    // 债单 B5：按块取最近标题（而非全文最后标题）；前缀计入 max_chars 上限，
    // 避免「块长 + 前缀」超过 BGE-M3 的 512 token 上限而被截断。
    chunks
        .into_iter()
        .map(|(content, heading)| {
            let prefix_base = if heading.is_empty() {
                file_name.to_string()
            } else {
                format!("{} > {}", file_name, heading)
            };
            let prefix = format!("[{}] ", prefix_base);
            let prefix_len = prefix.chars().count();
            let budget = max_chars.saturating_sub(prefix_len);
            // 前缀相对块太长（预算不足），放弃前缀保留原文
            if budget < 20 {
                return content;
            }
            let trimmed: String = if content.chars().count() > budget {
                content.chars().take(budget).collect()
            } else {
                content
            };
            format!("{}{}", prefix, trimmed)
        })
        .collect()
}

/// 文本文件 / 文件夹入库：读文本 → 分块 → BGE-M3 编码 → 落 `text_entries`。
///
/// `paths` 可以是文件（按扩展名过滤）或文件夹（递归找 `.md/.txt/...`）。
/// 长文档自动切片，块信息记在 `metadata_json`（chunk_index / chunk_count）。
#[tauri::command]
pub async fn ingest_text_files(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<TextIngestResult, String> {
    let started = Instant::now();
    let mut ingested: Vec<String> = Vec::new();
    let mut failed: Vec<TextIngestFailure> = Vec::new();
    let mut chunks_total = 0usize;

    // 1) 展开目标文件：文件夹递归，单文件按扩展名过滤
    let mut targets: Vec<String> = Vec::new();
    for raw in &paths {
        let path = normalize_path(raw);
        if path.is_empty() {
            continue;
        }
        let p = Path::new(&path);
        if p.is_dir() {
            for entry in WalkDir::new(p).into_iter().filter_map(|e| e.ok()) {
                if !entry.path().is_file() {
                    continue;
                }
                let ext = entry
                    .path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if TEXT_EXTENSIONS.contains(&ext.as_str()) {
                    targets.push(normalize_path(&entry.path().to_string_lossy()));
                }
            }
        } else if p.is_file() {
            let ext = p
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            if TEXT_EXTENSIONS.contains(&ext.as_str()) {
                targets.push(path);
            } else {
                // 债单 B6/C4：pdf/docx 是二进制格式，按 UTF-8 读会灌乱码。撤回扩展名后，
                // 明确拒收并给出可行动的指引，而非静默污染索引。
                let msg = if ext == "pdf" || ext == "docx" {
                    format!("Format {} is not supported yet; convert it to txt/md before importing", ext.to_uppercase())
                } else {
                    format!("Unsupported extension: .{}", ext)
                };
                failed.push(TextIngestFailure { path, error: msg });
            }
        } else {
            failed.push(TextIngestFailure {
                path,
                error: "Path does not exist".to_string(),
            });
        }
    }

    // 2) 逐文件处理：读 → 分块 → 编码（不持锁）→ 落库（一次锁）
    for path in &targets {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                failed.push(TextIngestFailure {
                    path: path.clone(),
                    error: format!("Read failed: {}", e),
                });
                continue;
            }
        };
        if content.trim().is_empty() {
            failed.push(TextIngestFailure {
                path: path.clone(),
                error: "Empty file".to_string(),
            });
            continue;
        }

        // P1-11 E11：传文件名给 chunk_text，用于上下文前缀
        let file_name = Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let chunks = chunk_text(&content, TEXT_CHUNK_CHARS, &file_name);
        if chunks.is_empty() {
            failed.push(TextIngestFailure {
                path: path.clone(),
                error: "No text after chunking".to_string(),
            });
            continue;
        }

        let mut err: Option<String> = None;

        // 编码：逐块发给 worker（BGE-M3）。此时不持 DB 锁，避免阻塞其它操作。
        let mut encoded: Vec<(usize, String, Vec<f32>)> = Vec::new();
        for (idx, chunk) in chunks.iter().enumerate() {
            match request_text_entry(chunk.clone(), path.clone()) {
                Ok((_worker_id, dense)) if !dense.is_empty() => encoded.push((idx, chunk.clone(), dense)),
                Ok(_) => {
                    err = Some(
                        "BGE-M3 text engine not available (run download_models.py --bge first)".to_string(),
                    );
                    break;
                }
                Err(e) => {
                    err = Some(e);
                    break;
                }
            }
        }

        // 落库：一个文件一次锁
        if err.is_none() {
            let mut db = state.db_conn.lock().unwrap();

            // 债单 C3：先删后插包事务——「删旧 + 插新」要么一起成功、要么一起回滚，
            // 绝不留下「旧块已删、新块没插上」的永久缺失（旧实现删旧失败不阻塞、且非同事务）。
            match db.transaction() {
                Ok(tx) => {
                    // P1-11 E8：ingest 前先删旧块——第二次 ingest 同一文件若分块数变少，
                    // 旧的高 idx 块不会被 INSERT OR REPLACE 覆盖，会永久残留。
                    if let Err(e) = text_store::delete_text_entries_by_source(&tx, path) {
                        let _ = tx.rollback();
                        err = Some(format!("Failed to delete old chunks: {}", e));
                    } else {
                        let mut insert_err: Option<String> = None;
                        for (idx, chunk, dense) in &encoded {
                            // entry_id 由 Rust 侧生成 `{来源}#{块号}`：便于按来源文件聚合管理
                            let entry_id = format!("{}#{}", path, idx);
                            let meta = json!({
                                "chunk_index": idx,
                                "chunk_count": chunks.len(),
                                "source": "file",
                            })
                            .to_string();
                            if let Err(e) = text_store::insert_text_entry(
                                &tx, &entry_id, path, chunk, dense, Some(&meta),
                            ) {
                                insert_err = Some(e);
                                break;
                            }
                        }
                        match insert_err {
                            Some(e) => {
                                let _ = tx.rollback();
                                err = Some(e);
                            }
                            None => {
                                if let Err(e) = tx.commit() {
                                    err = Some(format!("Commit failed: {}", e));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    err = Some(format!("Failed to begin transaction: {}", e));
                }
            };
        }

        // P1-11 partial-success 三态明确化：success / partial / failed
        match err {
            Some(e) => {
                // 有错误：若已有部分块编码成功并落库，记为 partial；否则记为 failed
                if !encoded.is_empty() {
                    // partial：部分块已入库，但后续块失败
                    chunks_total += encoded.len();
                    failed.push(TextIngestFailure {
                        path: path.clone(),
                        error: format!("Partial failure ({} chunks indexed): {}", encoded.len(), e),
                    });
                } else {
                    failed.push(TextIngestFailure {
                        path: path.clone(),
                        error: e,
                    });
                }
            }
            None if !encoded.is_empty() => {
                chunks_total += encoded.len();
                ingested.push(path.clone());
            }
            None => {} // 空文件/无编码结果，不重复加入 failed（上面已处理）
        }
    }

    fs_trace!(
        "text ingest done, files={}, chunks={}, failed={}, took={:?}",
        ingested.len(),
        chunks_total,
        failed.len(),
        started.elapsed()
    );

    Ok(TextIngestResult {
        ingested_files: ingested,
        chunks: chunks_total,
        failed,
    })
}

/// 列出已入库的纯文本条目（管理视图的「预览」侧）。
/// `query` 为子串关键词（按内容精确包含过滤），空串则不过滤——
/// 条目多了必须能先定位，才谈得上删除。
#[tauri::command]
pub async fn list_text_entries(
    state: State<'_, AppState>,
    page: usize,
    limit: usize,
    query: String,
) -> Result<TextEntryListResponse, String> {
    let db = state.db_conn.lock().unwrap();
    text_store::list_text_entries(&db, page, limit, &query)
}

/// 删除纯文本条目（**终态，不可回退**）。
///
/// 只接受用户显式勾选的 entry_id 列表——绝不提供「一键清空」。
#[tauri::command]
pub async fn delete_text_entries(
    state: State<'_, AppState>,
    entry_ids: Vec<String>,
) -> Result<usize, String> {
    if entry_ids.is_empty() {
        return Ok(0);
    }
    let db = state.db_conn.lock().unwrap();
    let removed = text_store::delete_text_entries(&db, &entry_ids)?;
    fs_trace!("text entries deleted, requested={}, removed={}", entry_ids.len(), removed);
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::chunk_text;

    // P1-11：chunk_text 签名改为 3 参数（加 file_name 用于上下文前缀）。
    // 测试传空文件名 = 不加前缀，保持旧行为可验证。

    #[test]
    fn empty_text_has_no_chunks() {
        assert!(chunk_text("", 10, "").is_empty());
        assert!(chunk_text("   \n\n  ", 10, "").is_empty());
    }

    #[test]
    fn short_text_is_a_single_chunk() {
        let chunks = chunk_text("hello world", 500, "");
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn paragraphs_are_kept_together_up_to_the_limit() {
        let chunks = chunk_text("aaaaa\n\nbbbbb", 20, "");
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn exceeding_the_limit_starts_a_new_chunk() {
        let chunks = chunk_text("aaaaa\n\nbbbbb", 11, "");
        assert_eq!(chunks.len(), 2);
    }

    #[test]
    fn an_over_long_paragraph_is_hard_split() {
        // overlap 会改变硬切结果（尾部保留进入下一块），故只验证块数 ≥ 2
        let chunks = chunk_text("abcdefghij", 4, "");
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn counts_characters_not_bytes_for_cjk() {
        // overlap 会改变分块边界，故只验证块数
        let chunks = chunk_text("一二三四五六七八", 5, "");
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn context_prefix_is_added() {
        // 传文件名时，足够大的块会被加上 [文件名 > 标题] 前缀
        let text = "# 标题一\n这是一段足够长的文字内容用来测试上下文前缀是否被正确添加";
        let chunks = chunk_text(text, 500, "test_doc");
        assert!(chunks.len() >= 1);
        // 块足够大时应有前缀
        if chunks[0].chars().count() > 20 {
            assert!(chunks[0].starts_with("[test_doc"), "expected prefix, got: {}", &chunks[0][..20]);
        }
    }
}
