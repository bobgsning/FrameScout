// SQLite 初始化、WAL 配置、表结构维护以及启动时载入内存。

use std::fs;
use rusqlite::{params, Connection};
use crate::constants::VECTOR_DIM;
use super::vector_matrix::FlatVectorMatrix;

pub fn init_db_and_load_memory() -> (Connection, FlatVectorMatrix) {
    println!("💾 Connecting to local SQLite Hybrid Matrix...");
    let app_data_dir = dirs::data_local_dir().unwrap().join("FrameScout-Offline_AI_Search-Global");
    if let Err(e) = fs::create_dir_all(&app_data_dir) {
        println!("⚠️ Warning: Failed to create AppData directory: {}", e);
    }

    let db_path = app_data_dir.join("framescout-offline_ai_search-global.db");
    // 声明为 mut：ensure_frame_vectors_schema 迁移时需要开启事务（&mut self）。
    let mut conn = Connection::open(&db_path).expect("Failed to open database");

    // WAL mode allows concurrent reads during writes
    let _ = conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;");

    // frame_vectors 表：确保 schema 为 (path, timestamp) 复合主键。
    // 该函数同时处理"全新建库"与"从旧版 path PRIMARY KEY 库迁移"两种情形。
    ensure_frame_vectors_schema(&mut conn);

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
                    if vector.len() == VECTOR_DIM {
                        memory_matrix.push(path, ts, vector, ocr_text, user_note, index_time);
                    } else {
                        println!("⚠️ Found obsolete vector (dim: {}) for path: {}. Marking for clean.", vector.len(), path);
                        stale_paths.push(path);
                    }
                }
            }
        }
    }

    // Clean up obsolete records。
    // 注意：复合主键下"按 path 删除"会删掉该文件的全部帧，符合"整文件维度过期即清理"的语义。
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
//  frame_vectors schema 维护
// =========================================================================
// 设计要点：
//   - 一个视频会被抽成多帧，每帧是一条独立记录，因此主键必须是 (path, timestamp)，
//     而非历史版本的单一 path PRIMARY KEY（旧模型下多帧会互相覆盖）。
//   - 向后兼容：对已存在的旧库（path 主键）做一次性迁移，复制现有数据（图片无损、
//     视频则保留旧库里那唯一一行——旧实现本就只存了首帧，无新增数据可补）。

/// frame_vectors 目标 schema 的 DDL（复合主键）。
const FRAME_VECTORS_SCHEMA_SQL: &str = "CREATE TABLE IF NOT EXISTS frame_vectors (
    path TEXT NOT NULL,
    timestamp REAL NOT NULL,
    vector_json TEXT NOT NULL,
    ocr_text TEXT DEFAULT '',
    user_note TEXT DEFAULT '',
    index_time REAL DEFAULT 0.0,
    PRIMARY KEY (path, timestamp)
)";

/// 判断指定表是否存在。
fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT name FROM sqlite_master WHERE type='table' AND name=?1",
        params![table],
        |_| Ok(true),
    )
    .unwrap_or(false)
}

/// 读取某表的主键列名集合（利用 PRAGMA table_info 的 pk 字段，pk>0 表示参与主键）。
fn primary_key_columns(conn: &Connection, table: &str) -> Vec<String> {
    let mut stmt = match conn.prepare(&format!("PRAGMA table_info({})", table)) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map([], |row| {
        let name: String = row.get(1)?;
        let pk: i32 = row.get(5)?;
        Ok((name, pk))
    });
    let mut pks = Vec::new();
    if let Ok(rows) = rows {
        for r in rows.flatten() {
            if r.1 > 0 {
                pks.push(r.0);
            }
        }
    }
    pks
}

/// 确认现有 frame_vectors 表的主键是否已经是 (path, timestamp) 复合键。
fn has_composite_key(conn: &Connection) -> bool {
    let pks = primary_key_columns(conn, "frame_vectors");
    pks.len() == 2
        && pks.iter().any(|c| c == "path")
        && pks.iter().any(|c| c == "timestamp")
}

/// 确保 frame_vectors 表存在且采用 (path, timestamp) 复合主键；
/// 对旧版以 path 为单一主键的库做一次性事务化迁移。
fn ensure_frame_vectors_schema(conn: &mut Connection) {
    // 全新库：直接以复合主键建表。
    if !table_exists(conn, "frame_vectors") {
        if let Err(e) = conn.execute(FRAME_VECTORS_SCHEMA_SQL, []) {
            println!("⚠️ Warning: Failed to create frame_vectors table: {}", e);
        }
        return;
    }

    // 已存在：若主键已是 (path, timestamp) 则无需处理。
    if has_composite_key(conn) {
        return;
    }

    // 旧库迁移：path PRIMARY KEY → (path, timestamp)。
    // 整个过程放在一个事务里，任意步骤失败即回滚，避免留下"表被改名但未建新表"
    // 的中间态。SQLite 的 DDL（含 ALTER TABLE RENAME）是事务性的，可安全回滚。
    println!("📦 Migrating frame_vectors: path PK → (path, timestamp) composite PK...");
    let tx = match conn.transaction() {
        Ok(tx) => tx,
        Err(e) => {
            println!("⚠️ frame_vectors migration aborted (cannot begin tx): {}", e);
            return;
        }
    };

    let migrate = (|| -> Result<(), rusqlite::Error> {
        // 1) 旧表让位
        tx.execute("ALTER TABLE frame_vectors RENAME TO frame_vectors_old", [])?;
        // 2) 建新表（复合主键）
        tx.execute(FRAME_VECTORS_SCHEMA_SQL, [])?;
        // 3) 拷贝存量数据：列名一一对应，旧库列结构与此一致。
        //    若未来增删列，需在此同步 SELECT/INSERT 的列清单。
        tx.execute(
            "INSERT INTO frame_vectors
                (path, timestamp, vector_json, ocr_text, user_note, index_time)
             SELECT path, timestamp, vector_json, ocr_text, user_note, index_time
             FROM frame_vectors_old",
            [],
        )?;
        // 4) 清理旧表
        tx.execute("DROP TABLE frame_vectors_old", [])?;
        Ok(())
    })();

    match migrate {
        Ok(_) => {
            if let Err(e) = tx.commit() {
                println!("⚠️ frame_vectors migration commit failed: {}", e);
            } else {
                println!("✅ frame_vectors migrated to (path, timestamp) composite key.");
            }
        }
        Err(e) => {
            // 回滚后旧表名恢复为 frame_vectors，应用可继续以旧 schema 运行（只是仍存不下多帧）。
            let _ = tx.rollback();
            println!("⚠️ frame_vectors migration failed, rolled back (running on legacy schema): {}", e);
        }
    }
}