// 负责智能文件夹的保存、列表统计、直接执行、重命名及删除。
//
// 第七轮 M011：智能文件夹新增 `criteria_json` —— 存搜索界面那条「多条件行」的序列化。
// 动机：旧表只有 (query_text + 4 个通道开关) 七个字段，保存时会**静默丢弃**
// 排除条件、分条件阈值、时间/目录/批次/类型条件 —— 用户摆好一排条件，存完语义就变了。
//
// 两条执行路径：
//  - `criteria_json` 为 NULL（旧文件夹）→ 前端把 7 字段映射成「快速关键词 + 通道选择」，
//    由前端条件引擎求值（走 search_unified，与搜索栏同一套）；
//  - `criteria_json` 非空 → **由前端条件引擎求值**（集合的交/差/并在前端内存里做），
//    后端不实现第二个求值器，也就不会有「两套语义」。此时 `match_count` 与
//    `refresh_smart_folder_count` 都给不出真值，UI 显示 `?`（needs_frontend_eval）。
// 第九轮：`execute_smart_folder` 与 `search_images` 已退役——智能文件夹不再走后端旧检索路径。

use rusqlite::params;
use tauri::State;
use crate::model_code::{AppState, SmartFolder};
use super::search_cmd::{match_text, normalize_query};
use super::search_unified_cmd::search_unified;

#[tauri::command]
pub async fn save_smart_folder(
    state: State<'_, AppState>,
    name: String,
    query_text: String,
    use_vector: bool,
    use_ocr: bool,
    use_note: bool,
    use_filename: bool,
    criteria_json: Option<String>,
) -> Result<i64, String> {
    let db = state.db_conn.lock().unwrap();
    db.execute(
        "INSERT INTO smart_folders (name, query_text, use_vector, use_ocr, use_note, use_filename, criteria_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            name,
            query_text,
            use_vector as i32,
            use_ocr as i32,
            use_note as i32,
            use_filename as i32,
            criteria_json
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(db.last_insert_rowid())
}

/// 更新智能文件夹（重命名 + 查询条件 + 目标开关 + 条件行）。
/// 不改 id，不改 match_count（match_count 由 get_smart_folders 动态计算）。
#[tauri::command]
pub async fn update_smart_folder(
    state: State<'_, AppState>,
    id: i64,
    name: String,
    query_text: String,
    use_vector: bool,
    use_ocr: bool,
    use_note: bool,
    use_filename: bool,
    criteria_json: Option<String>,
) -> Result<(), String> {
    let db = state.db_conn.lock().unwrap();
    db.execute(
        "UPDATE smart_folders
            SET name = ?1, query_text = ?2, use_vector = ?3, use_ocr = ?4,
                use_note = ?5, use_filename = ?6, criteria_json = ?7
         WHERE id = ?8",
        params![
            name,
            query_text,
            use_vector as i32,
            use_ocr as i32,
            use_note as i32,
            use_filename as i32,
            criteria_json,
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_smart_folders(state: State<'_, AppState>) -> Result<Vec<SmartFolder>, String> {
    let db = state.db_conn.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT id, name, query_text, use_vector, use_ocr, use_note, use_filename, criteria_json
                  FROM smart_folders ORDER BY id ASC")
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
            let criteria: Option<String> = row.get(7)?;
            Ok((id, name, query_text, v == 1, o == 1, n == 1, f == 1, criteria))
        })
        .map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    let lower_search_list: Vec<(i64, String, String, bool, bool, bool, bool, Option<String>)> =
        rows.filter_map(|r| r.ok()).collect();

    for (id, name, query_text, use_vector, use_ocr, use_note, use_filename, criteria_json) in
        lower_search_list
    {
        let mut count = 0;
        // 第九轮核查修复：列表计数此前用「整串 query 当连续子串」裸 contains，
        // 与执行路径的 `normalize_query`（切词 AND + 全角折叠）不是一套口径 ——
        // 多词 query（「白天 黑夜」）列表几乎必 0、执行按 AND 命中，数字失真。
        // 现改用与执行完全相同的 `normalize_query` + `match_text`，徽标与点进去一致。
        let tokens = normalize_query(&query_text);
        let has_query = !tokens.positive.is_empty();

        // **P1-3 / E1 修复**：旧实现循环体无 use_vector 分支，纯语义文件夹恒为 0。
        // 字面量通道（ocr/note/filename）走快速内存扫描；use_vector 通道无法在列表
        // 视图里实时跑（需要 ZMQ 编码 query + 全量向量搜索），前端用 `?` 表示
        // 「需执行才知道」，点进去走 execute_smart_folder 拿真实结果。
        // 这里加 use_vector 判定：若文件夹是纯语义（无字面量通道），count 留 0 让
        // 前端显示 `?`；若有混合通道，字面量计数作为下界（用户至少知道有多少条命中）。
        let has_text_channel = use_ocr || use_note || use_filename;

        if has_query && has_text_channel {
            for meta in &memory.metadata {
                let mut matched = false;
                if use_ocr && !meta.ocr_text.is_empty() && match_text(&meta.ocr_text.to_lowercase(), &tokens) {
                    matched = true;
                }
                if use_note && !meta.user_note.is_empty() && match_text(&meta.user_note.to_lowercase(), &tokens) {
                    matched = true;
                }
                if use_filename && match_text(&meta.path.to_lowercase(), &tokens) {
                    matched = true;
                }
                if matched {
                    count += 1;
                }
            }
        }

        // 有条件行 ⇒ 这个数字不是它的匹配数，别拿它冒充（前端据此显示 `?`）。
        let needs_frontend_eval = criteria_json.as_deref().is_some_and(|s| !s.trim().is_empty());
        if needs_frontend_eval {
            count = 0;
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
            criteria_json,
            needs_frontend_eval,
        });
    }

    Ok(list)
}

/// 按需刷新单个智能文件夹的真实计数（P1-3）。
/// 前端 hover 或点击 pill 时调用，走完整 search_unified 拿 total_count。
/// 对纯语义文件夹（use_vector-only）尤其重要——列表视图只能给 `?`，
/// 这里能给真实数字。代价是跑一次 ZMQ + 向量搜索，故按需而非列表批量调。
///
/// 第七轮：条件行文件夹（`needs_frontend_eval`）不在这里求值 —— 返回 0，
/// 由前端显示 `?`，点开时用条件引擎跑真结果。
#[tauri::command]
pub async fn refresh_smart_folder_count(
    state: State<'_, AppState>,
    id: i64,
) -> Result<usize, String> {
    let (query_text, use_vector, use_ocr, use_note, use_filename, criteria_json) = {
        let db = state.db_conn.lock().map_err(|e| e.to_string())?;
        db.query_row(
            "SELECT query_text, use_vector, use_ocr, use_note, use_filename, criteria_json
             FROM smart_folders WHERE id = ?1",
            params![id],
            |row| {
                let q: String = row.get(0)?;
                let v: i32 = row.get(1)?;
                let o: i32 = row.get(2)?;
                let n: i32 = row.get(3)?;
                let f: i32 = row.get(4)?;
                let c: Option<String> = row.get(5)?;
                Ok((q, v == 1, o == 1, n == 1, f == 1, c))
            },
        )
        .map_err(|e| format!("Smart folder not found: {}", e))?
    };

    if criteria_json.as_deref().is_some_and(|s| !s.trim().is_empty()) {
        // 条件行文件夹的交集/差集在前端算，后端给不出真值 —— 不编。
        return Ok(0);
    }

    // 第九轮核查修复：旧文件夹的「视觉 OR 字面」融合语义，search_unified 与之同口径
    // （is_hit = visual || ocr || note || filename），故计数改用 search_unified，
    // 与前端条件引擎执行完全一致 —— search_images 已退役。
    let result = search_unified(
        state,
        query_text,
        1,
        1, // 只要 total_count，limit=1 最省
        use_vector,
        use_ocr,
        use_note,
        use_filename,
        None,
        None,
        None,
        None,
        None,
    )
    .await?;

    Ok(result.total_count)
}

#[tauri::command]
pub async fn delete_smart_folder(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db_conn.lock().unwrap();
    db.execute("DELETE FROM smart_folders WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 智能文件夹导入（M2''' 补上「入」）：把导出的 JSON 定义写回 smart_folders 表。
/// 只接受「条件定义」字段（name + query_text + 4 布尔 + criteria_json），
/// 拒绝自增 id 与瞬时计数。
#[derive(serde::Deserialize)]
pub struct SmartFolderImportItem {
    pub name: String,
    pub query_text: String,
    pub use_vector: bool,
    pub use_ocr: bool,
    pub use_note: bool,
    pub use_filename: bool,
    /// 第七轮：条件行定义（可为 NULL）。
    #[serde(default)]
    pub criteria_json: Option<String>,
}

/// 导入结果：让前端能如实回报「导入了几条、其中几条改了名」。
#[derive(serde::Serialize)]
pub struct SmartFolderImportResult {
    pub imported: usize,
    /// 同名而自动改名（加 " (2)"、" (3)"…）的条数。
    pub renamed: usize,
    pub names: Vec<String>,
}

/// 第七轮核查修复：原先导入**完全不做查重**，重复导入同一个 JSON 会堆积一串同名文件夹，
/// 用户只能一个个删。现在按名字去重：撞名就加序号后缀，绝不覆盖已有定义。
#[tauri::command]
pub async fn import_smart_folders(
    state: State<'_, AppState>,
    folders: Vec<SmartFolderImportItem>,
) -> Result<SmartFolderImportResult, String> {
    let db = state.db_conn.lock().unwrap();
    let mut imported = 0usize;
    let mut renamed = 0usize;
    let mut names: Vec<String> = Vec::new();

    // 现有名字集合（一次性取回，避免逐条查询）
    let mut existing: std::collections::HashSet<String> = {
        let mut stmt = db
            .prepare("SELECT name FROM smart_folders")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.flatten().collect()
    };

    for f in folders {
        let base = f.name.trim();
        if base.is_empty() {
            continue;
        }
        let mut name = base.to_string();
        if existing.contains(&name) {
            renamed += 1;
            let mut n = 2;
            loop {
                let candidate = format!("{} ({})", base, n);
                if !existing.contains(&candidate) {
                    name = candidate;
                    break;
                }
                n += 1;
                if n > 999 {
                    name = format!("{} ({})", base, utc_stamp_suffix());
                    break;
                }
            }
        }
        existing.insert(name.clone());

        db.execute(
            "INSERT INTO smart_folders (name, query_text, use_vector, use_ocr, use_note, use_filename, criteria_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                name,
                f.query_text,
                f.use_vector as i32,
                f.use_ocr as i32,
                f.use_note as i32,
                f.use_filename as i32,
                f.criteria_json
            ],
        )
        .map_err(|e| e.to_string())?;
        imported += 1;
        names.push(name);
    }
    Ok(SmartFolderImportResult { imported, renamed, names })
}

/// 极端兜底：同名超过 999 条时的时间戳后缀。
fn utc_stamp_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "x".to_string())
}