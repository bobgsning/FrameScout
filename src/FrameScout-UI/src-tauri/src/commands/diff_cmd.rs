// 差异报告：比对「磁盘现状」与「库里的观察快照」，产出四组清单。
//
// 定位（写死）：
//   - 本命令**只做感知**：一次轻量预扫描（只 stat，不跑模型），产出清单。
//     它不写任何事件、不更新 `observed_*`、不自动索引任何东西。
//   - **行动由用户在明细层勾选后触发**，走已有命令：`index_files`（加入索引）、
//     `apply_ghost_action`（标记失效/真删除）。「忽略」则完全不落库——
//     它仅作用于本次批次，下次扫描该文件仍会出现，这正是设计意图：
//     `observed_*` 始终是纯客观感知信号，绝不被用户的临时决策污染。
//
// 数据源骨架：以「最近一次合法完成的扫描批次」为骨架（合法 = finished_at 非空），
// 叠加磁盘现状的差异。本次只审查给定根目录，该目录之外的库记录不会被误判为「消失」。

use std::collections::{HashMap, HashSet};
use std::path::Path;

use tauri::State;
use walkdir::WalkDir;

use crate::constants::{IMAGE_EXTENSIONS, VIDEO_EXTENSIONS};
use crate::fs_trace;
use crate::model_code::{AppState, DiffItem, DiffKind, DiffReport};
use crate::storage::journal::{self, BatchType, FileStat};
use crate::storage::normalize_path;

/// 判定「疑似外部同步」的门槛：
/// 修改项达到 5 个以上，且占本次扫描文件的 30% 以上。
/// 只用于给出提示文案，绝不自动做任何处理。
const MASS_CHANGE_MIN_COUNT: usize = 5;
const MASS_CHANGE_MIN_RATIO_PERCENT: usize = 30;

/// 生成差异报告（只读）。
#[tauri::command]
pub async fn scan_diff(
    state: State<'_, AppState>,
    folder_path: String,
) -> Result<DiffReport, String> {
    if folder_path.trim().is_empty() {
        return Err("Pick a folder to review first.".to_string());
    }

    let db = state.db_conn.lock().map_err(|e| e.to_string())?;

    let baseline_batch_id = journal::latest_completed_batch(&db, BatchType::Scan);

    // ① 磁盘现状：一次轻量 WalkDir + stat（不解码、不推理）
    let mut disk: HashMap<String, FileStat> = HashMap::new();
    for entry in WalkDir::new(&folder_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !IMAGE_EXTENSIONS.contains(&ext.as_str()) && !VIDEO_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }
        let key = normalize_path(&path.to_string_lossy());
        if key.is_empty() {
            continue;
        }
        if let Some(stat) = journal::stat_file(&key) {
            disk.insert(key, stat);
        }
    }
    let scanned_files = disk.len();

    // ② 库里的观察快照（含向量行数），一次取全，避免逐路径查询
    #[derive(Default)]
    struct Snapshot {
        mtime: Option<i64>,
        size: Option<i64>,
        frame_count: usize,
        last_seen_at: Option<String>,
        is_dead: bool,
    }

    let mut snapshots: HashMap<String, Snapshot> = HashMap::new();
    {
        let mut stmt = db
            .prepare(
                "SELECT f.path, f.observed_mtime, f.observed_size, f.is_dead, f.last_seen_at,
                        (SELECT COUNT(*) FROM frame_vectors fv WHERE fv.path = f.path) AS frame_count
                 FROM files f",
            )
            .map_err(|e| format!("Prepare error: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, i64>(3)? != 0,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)? as usize,
                ))
            })
            .map_err(|e| format!("Query error: {}", e))?;

        for row in rows.flatten() {
            let (path, mtime, size, is_dead, last_seen_at, frame_count) = row;
            snapshots.insert(
                path,
                Snapshot {
                    mtime,
                    size,
                    frame_count,
                    last_seen_at,
                    is_dead,
                },
            );
        }
    }

    let mut items: Vec<DiffItem> = Vec::new();

    // ③ 新增 / 修改：以磁盘为准逐条比对
    let mut disk_paths: Vec<&String> = disk.keys().collect();
    disk_paths.sort();

    for path in disk_paths {
        let stat = disk[path];
        match snapshots.get(path) {
            None => {
                items.push(DiffItem {
                    path: path.clone(),
                    kind: DiffKind::New,
                    observed_mtime: None,
                    observed_size: None,
                    disk_mtime: Some(stat.mtime_ms),
                    disk_size: Some(stat.size),
                    frame_count: 0,
                    last_seen_at: None,
                });
            }
            Some(snap) => {
                // 复用 journal 的阈值判定，保证「感知」口径全局一致
                let observation = journal::classify_change(snap.mtime, snap.size, Some(stat));
                if observation == journal::Observation::Changed {
                    items.push(DiffItem {
                        path: path.clone(),
                        kind: DiffKind::Modified,
                        observed_mtime: snap.mtime,
                        observed_size: snap.size,
                        disk_mtime: Some(stat.mtime_ms),
                        disk_size: Some(stat.size),
                        frame_count: snap.frame_count,
                        last_seen_at: snap.last_seen_at.clone(),
                    });
                }
            }
        }
    }

    // ④ 消失：本次扫描目录内、库中仍有锚点、但磁盘上找不到了
    let root_key = normalize_path(&folder_path);
    let disk_set: HashSet<&String> = disk.keys().collect();
    for (path, snap) in &snapshots {
        if snap.is_dead || disk_set.contains(path) {
            continue;
        }
        if !path.starts_with(&root_key) {
            continue; // 目录之外的记录不参与本次判定，避免误报
        }
        items.push(DiffItem {
            path: path.clone(),
            kind: DiffKind::Missing,
            observed_mtime: snap.mtime,
            observed_size: snap.size,
            disk_mtime: None,
            disk_size: None,
            frame_count: snap.frame_count,
            last_seen_at: snap.last_seen_at.clone(),
        });
    }

    // ⑤ 需要索引但无向量：常驻第四组，与目录无关（全局契约）
    for (path, snap) in &snapshots {
        if snap.is_dead || snap.frame_count > 0 {
            continue;
        }
        items.push(DiffItem {
            path: path.clone(),
            kind: DiffKind::Unindexed,
            observed_mtime: snap.mtime,
            observed_size: snap.size,
            disk_mtime: disk.get(path).map(|s| s.mtime_ms),
            disk_size: disk.get(path).map(|s| s.size),
            frame_count: 0,
            last_seen_at: snap.last_seen_at.clone(),
        });
    }

    items.sort_by(|a, b| a.kind.as_str().cmp(b.kind.as_str()).then_with(|| a.path.cmp(&b.path)));

    let new_count = items.iter().filter(|i| i.kind == DiffKind::New).count();
    let modified_count = items.iter().filter(|i| i.kind == DiffKind::Modified).count();
    let missing_count = items.iter().filter(|i| i.kind == DiffKind::Missing).count();
    let unindexed_count = items.iter().filter(|i| i.kind == DiffKind::Unindexed).count();

    let mass_change_suspected = modified_count >= MASS_CHANGE_MIN_COUNT
        && scanned_files > 0
        && modified_count * 100 / scanned_files >= MASS_CHANGE_MIN_RATIO_PERCENT;

    fs_trace!(
        "scan diff, root={:?}, baseline={:?}, scanned={}, new={}, modified={}, missing={}, unindexed={}, mass_change={}",
        Path::new(&root_key).file_name().map(|s| s.to_string_lossy().to_string()),
        baseline_batch_id,
        scanned_files,
        new_count,
        modified_count,
        missing_count,
        unindexed_count,
        mass_change_suspected
    );

    Ok(DiffReport {
        baseline_batch_id,
        root: root_key,
        scanned_files,
        mass_change_suspected,
        new_count,
        modified_count,
        missing_count,
        unindexed_count,
        items,
    })
}
