// =========================================================================
//  schema 版本与迁移链（v3.2.0 工程纪律）
// =========================================================================
//  约定（写死，后续一律遵守）：
//   1. 版本号存放在 SQLite 的 `PRAGMA user_version`。0 表示「从未打过版本戳」，
//      涵盖全新库与 v3.1.x 及更早的历史库。
//   2. 每个迁移显式声明 from_version / to_version，按数组顺序串行执行；
//      迁移函数自身必须**幂等**（重复执行不报错、不产生副作用），
//      因此可从任一历史版本一路升到最新。
//   3. **备份绑定迁移，不绑定启动**：仅当「代码期望版本 > 库当前版本」时才做一次
//      迁移前备份；版本一致时零 IO。备份不是每次启动的仪式。
//   4. 一次启动只跑一条迁移链：proto Version 与 DB schema 变更登记在同一条链上。
//
//  新增迁移的正确姿势：
//   - SCHEMA_VERSION += 1；
//   - 在 MIGRATIONS 末尾追加一项（from = 旧 SCHEMA_VERSION，to = 新值）；
//   - 迁移函数内部先做「是否已满足目标形态」的判断，不满足才动手。

use std::fs;
use std::path::Path;

use rusqlite::{params, Connection, Transaction};

use crate::time_util::{utc_now_iso, utc_stamp};
use super::journal;

/// 当前代码期望的 schema 版本。必须等于 MIGRATIONS 最后一项的 to_version。
pub const SCHEMA_VERSION: u32 = 9;

/// frame_vectors 的最新目标 DDL。全新库（M001）与重建表（M002/M003）共用同一份，
/// 杜绝「两处 DDL 各写一份、日后漂移」的问题。
///
/// 列语义：
///  - `vector_json`：遗留列。**仅旧行有值**，读取时作为 BLOB 缺失时的回退；
///    新写入一律不再填充（不为旧格式支付双写成本），故可为 NULL。
///    ⚠️ 未来「把 JSON 全量转 BLOB 后 DROP」的清整迁移必须注意：
///    已被重索引过的旧行，其 BLOB 是新的、JSON 是旧的，**只可转换
///    `vector_f32 IS NULL` 的行**，绝不能用 JSON 覆盖已有的 BLOB。
///  - `vector_f32`：新写入的唯一向量落点，小端序平面 f32（见 vector_blob.rs）。
///  - `indexed_in_batch_id`：占位列，扫描入库时顺手写入当前批次名
///    （`scan_<UTC>` / `clean_ghosts_<UTC>` / `legacy_import`），
///    为「按批次重编向量、模型升级回溯、按批次清理」预埋，少一跳 join。
const FRAME_VECTORS_DDL: &str = "CREATE TABLE IF NOT EXISTS frame_vectors (
    path TEXT NOT NULL,
    timestamp REAL NOT NULL,
    vector_json TEXT,
    vector_f32 BLOB,
    ocr_text TEXT DEFAULT '',
    user_note TEXT DEFAULT '',
    index_time REAL DEFAULT 0.0,
    indexed_in_batch_id TEXT,
    PRIMARY KEY (path, timestamp)
)";

struct Migration {
    from_version: u32,
    to_version: u32,
    name: &'static str,
    apply: fn(&Transaction) -> rusqlite::Result<()>,
}

/// 迁移链。顺序即执行顺序，不可重排。
const MIGRATIONS: &[Migration] = &[
    Migration {
        from_version: 0,
        to_version: 1,
        name: "baseline_schema_v32",
        apply: m001_baseline_schema,
    },
    Migration {
        from_version: 1,
        to_version: 2,
        name: "frame_vectors_composite_pk",
        apply: m002_composite_primary_key,
    },
    Migration {
        from_version: 2,
        to_version: 3,
        name: "frame_vectors_blob_columns",
        apply: m003_blob_columns,
    },
    Migration {
        from_version: 3,
        to_version: 4,
        name: "ledger_tables",
        apply: m004_ledger_tables,
    },
    Migration {
        from_version: 4,
        to_version: 5,
        name: "reindex_backup",
        apply: m005_reindex_backup,
    },
    Migration {
        from_version: 5,
        to_version: 6,
        name: "ocr_entries",
        apply: m006_ocr_entries,
    },
    Migration {
        from_version: 6,
        to_version: 7,
        name: "text_vectors",
        apply: m007_text_vectors,
    },
    Migration {
        from_version: 7,
        to_version: 8,
        name: "performance_indexes",
        apply: m008_performance_indexes,
    },
    Migration {
        from_version: 8,
        to_version: 9,
        name: "fts5_trigram",
        apply: m009_fts5_trigram,
    },
];

// =========================================================================
//  对外入口
// =========================================================================

/// 读取库当前的 schema 版本（PRAGMA user_version）。
pub fn current_schema_version(conn: &Connection) -> u32 {
    conn.query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
        .unwrap_or(0)
}

/// 按序执行迁移链。版本已是最新时直接返回（零 IO）。
///
/// `db_path` 用于迁移前备份（备份文件落在同目录的 `backups/` 下）。
pub fn run_migrations(conn: &mut Connection, db_path: &Path) {
    let mut version = current_schema_version(conn);

    if version >= SCHEMA_VERSION {
        println!("💾 Schema up to date (user_version = {}). No migration needed.", version);
        return;
    }

    // 进入结构变更期。库里已有业务数据时才备份——全新库无数据可保护。
    if table_exists(conn, "frame_vectors") {
        backup_before_migrate(conn, db_path, version);
    }

    for m in MIGRATIONS {
        if version >= m.to_version {
            continue;
        }
        if version != m.from_version {
            println!(
                "⚠️ Migration chain broken: user_version = {} but next migration expects {} ({}). Aborting.",
                version, m.from_version, m.name
            );
            return;
        }

        println!(
            "📦 Applying migration {} → {} ({})...",
            m.from_version, m.to_version, m.name
        );

        let tx = match conn.transaction() {
            Ok(tx) => tx,
            Err(e) => {
                println!("⚠️ Cannot begin transaction for {}: {}", m.name, e);
                return;
            }
        };

        match (m.apply)(&tx) {
            Ok(_) => match set_schema_version(&tx, m.to_version) {
                Ok(_) => match tx.commit() {
                    Ok(_) => {
                        version = m.to_version;
                        println!("✅ Migration applied: {} (user_version = {}).", m.name, version);
                    }
                    Err(e) => println!("⚠️ Commit failed for {}: {}", m.name, e),
                },
                Err(e) => println!("⚠️ Version stamp failed for {}: {}", m.name, e),
            },
            Err(e) => {
                // 迁移失败即回滚。旧库仍可继续以旧 schema 运行，不留半截状态。
                let _ = tx.rollback();
                println!("⚠️ Migration {} failed, rolled back: {}", m.name, e);
                return;
            }
        }
    }

    if version >= SCHEMA_VERSION {
        println!("✅ Schema migrated to version {}.", version);
    }
}

// =========================================================================
//  迁移实现
// =========================================================================

/// M001 · 基线 schema：所有基础表一律 `CREATE TABLE IF NOT EXISTS`，天然幂等。
/// 全新库直接以最新结构建表，因此后续迁移对全新库而言全部 no-op。
fn m001_baseline_schema(tx: &Transaction) -> rusqlite::Result<()> {
    tx.execute_batch(&format!(
        "{};
        CREATE TABLE IF NOT EXISTS smart_folders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            query_text TEXT NOT NULL,
            use_vector INTEGER DEFAULT 1,
            use_ocr INTEGER DEFAULT 1,
            use_note INTEGER DEFAULT 1,
            use_filename INTEGER DEFAULT 1
        );",
        FRAME_VECTORS_DDL
    ))
}

/// M002 · 历史库（v3.1.0 之前）的 `path` 单主键 → `(path, timestamp)` 复合主键。
///
/// 幂等性来自两道判断：表不存在（全新库）或已是复合主键（已迁移）时直接 no-op。
/// 迁移本体是 rename → create → copy → drop，全程单事务，失败可回滚。
fn m002_composite_primary_key(tx: &Transaction) -> rusqlite::Result<()> {
    if !table_exists(tx, "frame_vectors") {
        return Ok(());
    }
    if has_composite_key(tx) {
        return Ok(());
    }

    println!("📦 Migrating frame_vectors: path PK → (path, timestamp) composite PK...");

    tx.execute("ALTER TABLE frame_vectors RENAME TO frame_vectors_old", [])?;
    // 直接以最新结构建表：旧库一次升级即可拿到全部新列，后续迁移自然 no-op。
    tx.execute_batch(FRAME_VECTORS_DDL)?;
    // 列名一一对应。若将来增删列，需在此同步 SELECT/INSERT 的列清单。
    tx.execute(
        "INSERT INTO frame_vectors
            (path, timestamp, vector_json, ocr_text, user_note, index_time)
         SELECT path, timestamp, vector_json, ocr_text, user_note, index_time
         FROM frame_vectors_old",
        [],
    )?;
    tx.execute("DROP TABLE frame_vectors_old", [])?;
    Ok(())
}

/// M003 · 为已有复合主键的库补上 `vector_f32` / `indexed_in_batch_id`，
/// 并把遗留的 `vector_json` 列解除 NOT NULL。
///
/// 为什么必须重建表而不是只 ALTER：
///  新写入不再填充 `vector_json`（旧格式是一次性消耗品，不支付双写成本），
///  而旧 schema 里它是 `TEXT NOT NULL`——不指定该列的 INSERT 会被约束拒绝。
///  SQLite 的 ALTER TABLE 无法修改列约束，因此这里沿用 rename→create→copy→drop。
///
/// 幂等性：`vector_f32` 列已存在（全新库或已走过 M002）时直接 no-op。
fn m003_blob_columns(tx: &Transaction) -> rusqlite::Result<()> {
    if !table_exists(tx, "frame_vectors") {
        return Ok(());
    }
    if has_column(tx, "frame_vectors", "vector_f32") {
        return Ok(());
    }

    println!("📦 Migrating frame_vectors: adding vector_f32 BLOB / indexed_in_batch_id...");
    tx.execute("ALTER TABLE frame_vectors RENAME TO frame_vectors_old", [])?;
    tx.execute_batch(FRAME_VECTORS_DDL)?;
    // 旧行的 vector_json 原样搬过去，作为 BLOB 缺失时的读取回退；
    // 新列（vector_f32 / indexed_in_batch_id）留 NULL。
    tx.execute(
        "INSERT INTO frame_vectors
            (path, timestamp, vector_json, ocr_text, user_note, index_time)
         SELECT path, timestamp, vector_json, ocr_text, user_note, index_time
         FROM frame_vectors_old",
        [],
    )?;
    tx.execute("DROP TABLE frame_vectors_old", [])?;
    Ok(())
}

/// M004 · 建立账簿三表（files / batches / file_events），并为存量库补写基线。
///
/// 三表均为新增，不触碰 `frame_vectors`，因此搜索链路零影响。
///
/// 基线补写顺序（避免 `files` 空表时的鸡生蛋问题）：
///   1. 遍历 `frame_vectors` 现有 path（**逐路径一次，不逐帧**，避免视频噪音）；
///   2. 为每条路径在 `files` upsert 一行（能 stat 到就填真实 mtime/size，
///      否则留 NULL；`is_dead` 一律 0——看不到不等于已失效，不自作主张）；
///   3. 以 `batch_id = 'legacy_import'` 逐路径写一条 `first_seen` 事件。
///
/// 幂等性：仅在「`files` 表为空且 `frame_vectors` 非空」时补写，
/// 因此重复执行不会产生重复基线。
fn m004_ledger_tables(tx: &Transaction) -> rusqlite::Result<()> {
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS files (
            path TEXT PRIMARY KEY,
            observed_mtime INTEGER,
            observed_size INTEGER,
            is_dead INTEGER DEFAULT 0,
            dead_at TEXT,
            first_seen_at TEXT,
            last_seen_at TEXT
        );
        CREATE TABLE IF NOT EXISTS batches (
            id INTEGER PRIMARY KEY,
            batch_id TEXT UNIQUE,
            batch_type TEXT,
            started_at TEXT,
            finished_at TEXT,
            status TEXT,
            summary_json TEXT
        );
        CREATE TABLE IF NOT EXISTS file_events (
            id INTEGER PRIMARY KEY,
            batch_id TEXT,
            path TEXT,
            event_type TEXT,
            occurred_at TEXT,
            payload_json TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_file_events_batch ON file_events (batch_id);
        CREATE INDEX IF NOT EXISTS idx_file_events_path  ON file_events (path);
        CREATE INDEX IF NOT EXISTS idx_file_events_time  ON file_events (occurred_at);",
    )?;

    seed_legacy_baseline(tx)
}

/// M005 · 重索引前的向量冷备份表。
///
/// 为什么必须现在建：**快照只能在 DELETE 之前存**。今天不存，这批历史向量就
/// 永远消失，将来想补救也补不回来（now-or-never 的不对称性）。
///
/// 明确不做的另一半：撤销命令、UI 入口、让备份向量参与检索。
/// 本表**不进内存矩阵**，只是冷存储——它让「重索引不可回退」从
/// 「数据没了」降级为「默认不回退，但数据还在，必要时可人工恢复」。
///
/// 语义不受影响：`dead` = 保留可复活；`reindex` = 替换不可回退。
/// 备份表不改变这两条，只是给第二条留了条退路。
fn m005_reindex_backup(tx: &Transaction) -> rusqlite::Result<()> {
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS reindex_backup (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL,
            timestamp REAL NOT NULL,
            vector_f32 BLOB,
            vector_json TEXT,
            ocr_text TEXT,
            user_note TEXT,
            index_time REAL,
            backup_batch_id TEXT,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_reindex_backup_path ON reindex_backup(path, created_at);",
    )
}

/// M006 · OCR 行级结果落库（数据先存、高亮后做）。
///
/// `ocr_text` 保留为拼接全文（兼容旧搜索），但**检索与定位以本表为真数据源**。
/// `timestamp` 列让同一 path（视频）下多帧 OCR 各自归属；图片恒为 0。
/// 坐标归一化 0~1 存储，避免分辨率依赖；原始四角另存 payload_json，
/// 供未来精确框选（初版高亮用直角框即可）。
fn m006_ocr_entries(tx: &Transaction) -> rusqlite::Result<()> {
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS media_ocr_entries (
            path TEXT,
            timestamp INTEGER DEFAULT 0,
            text_chunk TEXT,
            bbox_left REAL, bbox_top REAL, bbox_right REAL, bbox_bottom REAL,
            conf REAL,
            payload_json TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_media_ocr_path ON media_ocr_entries(path, timestamp);",
    )
}

/// M007 · 文本通道的两张表（BGE-M3 的落点，B 线第 4 批）。
///
/// 为什么分两张表（协议 4.6 已钉死的两类文本必须分开）：
///   - `media_text_vectors` —— 媒体**内嵌文字**（OCR 文本）的 BGE-M3 向量。
///     它是媒体的**属性**，用户意图是「找到这个画面」；
///   - `text_entries`      —— 纯文本条目（笔记 / 文档 / 外部转录）的向量。
///     它是**独立检索对象**，用户意图是「找到这段内容本身」。
///   二者向量空间一致（都出自 BGE-M3），但检索价值异构，混在一张表会翻车
///   （搜「什么是爱情」会召回一堆电影字幕与路牌）。
///
/// 混库禁忌：`model` 列标出向量出自哪个模型，检索时必须**严格按 model 过滤，
/// 绝不跨模型算相似度**。本表是纯新增，不触碰 frame_vectors / 账簿 / OCR 表。
///
/// dense_blob 字节序约定（写死）：小端序平面 f32，BGE-M3 dense 为 1024 维即
/// 4096 字节，无 header 无 padding——与 frame_vectors.vector_f32 同一套约定
/// （见 vector_blob.rs），复用同一组 encode/decode 函数，只是维度参数不同。
fn m007_text_vectors(tx: &Transaction) -> rusqlite::Result<()> {
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS media_text_vectors (
            path TEXT NOT NULL,
            timestamp INTEGER NOT NULL DEFAULT 0,  -- 视频帧时间戳（毫秒）；图片恒为 0
            model TEXT NOT NULL,                   -- 'bge-m3' 等；混库时严格按此过滤
            dense_blob BLOB,                       -- BGE-M3 dense，1024×4=4096 字节，小端 f32
            sparse_json TEXT,                      -- 词 -> 权重（JSON），供精确匹配 / 未来 FTS5
            chunk_index INTEGER NOT NULL DEFAULT 0,-- 长文本切片序号（通常为 0）
            parent_id TEXT,                        -- 切片归属，可空
            PRIMARY KEY (path, timestamp, chunk_index)
        );
        CREATE INDEX IF NOT EXISTS idx_media_text_vectors_model ON media_text_vectors (model);

        CREATE TABLE IF NOT EXISTS text_entries (
            entry_id TEXT PRIMARY KEY,
            source_uri TEXT,          -- 来源标识（非文件系统绝对路径；音频可填 audio_memo）
            content TEXT,             -- 原文
            model TEXT NOT NULL,      -- 'bge-m3'
            dense_blob BLOB,          -- 1024×4 字节，小端 f32
            sparse_json TEXT,         -- 词 -> 权重
            index_time REAL,          -- Unix 秒
            metadata_json TEXT        -- 标签 / 语言 / 作者
        );
        CREATE INDEX IF NOT EXISTS idx_text_entries_source ON text_entries (source_uri);
        CREATE INDEX IF NOT EXISTS idx_text_entries_time ON text_entries (index_time);",
    )
}

/// M008 · 性能索引（第三轮 P1-15 / C3 / D16）。
///
/// 旧库只有 file_events(batch_id/path/occurred_at)、reindex_backup(path,created_at)、
/// media_ocr_entries(path,timestamp)、media_text_vectors(model)、text_entries 的索引，
/// files 表与 frame_vectors.index_time 全无索引 ⇒ 启动 `ORDER BY index_time DESC` 全表 filesort、
/// ghost_cmd 的 `ORDER BY last_seen_at DESC` 也走 filesort。
///
/// 本迁移纯加索引、不改表结构，幂等安全（CREATE INDEX IF NOT EXISTS）：
///   - files(is_dead)             : ghost_cmd 与 search_text 过滤失效文件
///   - files(last_seen_at)        : ghost_cmd 的 ORDER BY last_seen_at DESC
///   - frame_vectors(index_time)  : 启动载入与 list_all_files 的 ORDER BY index_time DESC
///   - batches(finished_at)       : latest_completed_batch 与差异报告的合法批次筛选
fn m008_performance_indexes(tx: &Transaction) -> rusqlite::Result<()> {
    tx.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_files_is_dead ON files (is_dead);
        CREATE INDEX IF NOT EXISTS idx_files_last_seen_at ON files (last_seen_at);
        CREATE INDEX IF NOT EXISTS idx_frame_vectors_index_time ON frame_vectors (index_time);
        CREATE INDEX IF NOT EXISTS idx_batches_finished_at ON batches (finished_at);",
    )
}

/// M009 · FTS5 trigram 容错检索（P2-1 / 第三轮 A6/E13）。
///
/// 旧实现：`text_store.rs` 的 `list_text_entries` 用 `LIKE '%query%'` 做子串匹配，
/// 前导通配符导致索引失效全表扫，且无法容错（少一个字就搜不到）。
/// FTS5 trigram tokenizer 把文本切成 3 字符 gram 建索引，支持 MATCH 查询与容错匹配，
/// 对 CJK 友好（中文 3 字 = 一个 trigram，恰好是词级粒度）。
///
/// 此迁移：
///   1. 创建 `text_entries_fts` 虚拟表（FTS5 + trigram），列含 content + entry_id（外键）；
///   2. 创建触发器：INSERT/UPDATE/DELETE text_entries 时同步 fts 表；
///   3. 回填存量数据：把已有 text_entries 的 content 灌进 fts 表。
fn m009_fts5_trigram(tx: &Transaction) -> rusqlite::Result<()> {
    // 1. 创建虚拟表。trigram tokenizer 需要编译进 SQLite（rusqlite 默认 bundled 含 FTS5）。
    tx.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS text_entries_fts USING fts5(
            content,
            entry_id UNINDEXED,
            tokenize = 'trigram'
        );",
    )?;

    // 2. 触发器：同步 text_entries → fts 表
    tx.execute_batch(
        "CREATE TRIGGER IF NOT EXISTS trg_text_entries_ai AFTER INSERT ON text_entries BEGIN
            INSERT INTO text_entries_fts (content, entry_id) VALUES (new.content, new.entry_id);
        END;
        CREATE TRIGGER IF NOT EXISTS trg_text_entries_ad AFTER DELETE ON text_entries BEGIN
            DELETE FROM text_entries_fts WHERE entry_id = old.entry_id;
        END;
        CREATE TRIGGER IF NOT EXISTS trg_text_entries_au AFTER UPDATE ON text_entries BEGIN
            DELETE FROM text_entries_fts WHERE entry_id = old.entry_id;
            INSERT INTO text_entries_fts (content, entry_id) VALUES (new.content, new.entry_id);
        END;",
    )?;

    // 3. 回填存量数据（首次迁移时把已有条目灌进 fts）
    let count: i64 = tx.query_row("SELECT COUNT(*) FROM text_entries", [], |r| r.get(0))?;
    if count > 0 {
        tx.execute_batch(
            "INSERT OR IGNORE INTO text_entries_fts (content, entry_id)
             SELECT content, entry_id FROM text_entries;",
        )?;
    }

    Ok(())
}

/// 为 v3.2 之前的存量库补 `files` 锚点与 `first_seen` 事件。
fn seed_legacy_baseline(tx: &Transaction) -> rusqlite::Result<()> {
    // 只在「锚点表为空、但库里确有存量数据」时执行，天然幂等。
    let files_empty: bool = tx.query_row("SELECT COUNT(*) FROM files", [], |r| r.get::<_, i64>(0))? == 0;
    if !files_empty {
        return Ok(());
    }
    if !table_exists(tx, "frame_vectors") {
        return Ok(());
    }

    let mut stmt = tx.prepare("SELECT DISTINCT path FROM frame_vectors")?;
    let paths: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .flatten()
        .collect();
    drop(stmt);

    if paths.is_empty() {
        return Ok(());
    }

    let now = utc_now_iso();
    let legacy_id = journal::BatchType::LegacyImport.as_str();
    // 存量导入本身也是一个合法完成的批次：它的骨架让「这批 first_seen 从哪来」有据可查。
    tx.execute(
        "INSERT OR IGNORE INTO batches (batch_id, batch_type, started_at, finished_at, status, summary_json)
         VALUES (?1, ?2, ?3, ?3, 'completed', ?4)",
        params![
            legacy_id,
            legacy_id,
            now,
            serde_json::json!({ "seeded_paths": paths.len() }).to_string()
        ],
    )?;

    for path in &paths {
        let stat = journal::stat_file(path);
        journal::upsert_file_observed(tx, path, stat).map_err(to_sqlite_error)?;
        journal::record_event(tx, legacy_id, path, journal::EventType::FirstSeen, None)
            .map_err(to_sqlite_error)?;
    }

    println!(
        "📒 Legacy baseline seeded: {} paths into files + first_seen events (batch_id = legacy_import).",
        paths.len()
    );
    Ok(())
}

/// 把 journal 层的 String 错误转成 rusqlite::Error，以便纳入迁移事务的统一回滚。
fn to_sqlite_error(msg: String) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(msg)))
}

// =========================================================================
//  备份（仅在结构变更期执行一次）
// =========================================================================

/// 迁移前把主库文件复制一份到 `<db_dir>/backups/pre_migrate_<UTC时间戳>.db`。
/// 复制前先 `wal_checkpoint(TRUNCATE)`，确保 WAL 中已提交的数据全部落进主库文件。
fn backup_before_migrate(conn: &Connection, db_path: &Path, from_version: u32) {
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

    let db_dir = match db_path.parent() {
        Some(d) => d,
        None => return,
    };
    let backup_dir = db_dir.join("backups");
    if let Err(e) = fs::create_dir_all(&backup_dir) {
        println!("⚠️ Cannot create backup dir {:?}: {}", backup_dir, e);
        return;
    }

    let target = backup_dir.join(format!("pre_migrate_v{}_{}.db", from_version, utc_stamp()));
    match fs::copy(db_path, &target) {
        Ok(size) => println!("🗂️ Pre-migration backup written: {:?} ({} bytes)", target, size),
        Err(e) => println!("⚠️ Pre-migration backup failed: {}", e),
    }
}

// =========================================================================
//  小工具（供迁移函数使用）
// =========================================================================

/// 判断指定表是否存在。
pub fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT name FROM sqlite_master WHERE type='table' AND name=?1",
        params![table],
        |_| Ok(true),
    )
    .unwrap_or(false)
}

/// 判断指定表是否含有某列（迁移幂等判断的基础手段）。
pub fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
    table_columns(conn, table).iter().any(|c| c.name == column)
}

#[derive(Debug)]
struct ColumnInfo {
    name: String,
    is_pk: bool,
}

/// 读取某表的列信息（PRAGMA table_info：name 在第 2 列，pk 在第 6 列）。
fn table_columns(conn: &Connection, table: &str) -> Vec<ColumnInfo> {
    let mut stmt = match conn.prepare(&format!("PRAGMA table_info({})", table)) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map([], |row| {
        Ok(ColumnInfo {
            name: row.get(1)?,
            is_pk: row.get::<_, i32>(5)? > 0,
        })
    });
    match rows {
        Ok(rows) => rows.flatten().collect(),
        Err(_) => Vec::new(),
    }
}

/// 现有 frame_vectors 的主键是否已是 (path, timestamp) 复合键。
fn has_composite_key(conn: &Connection) -> bool {
    let cols = table_columns(conn, "frame_vectors");
    let pks: Vec<&str> = cols.iter().filter(|c| c.is_pk).map(|c| c.name.as_str()).collect();
    pks.len() == 2 && pks.contains(&"path") && pks.contains(&"timestamp")
}

/// 写入 schema 版本号。与迁移本体同事务，保证「结构变了但版本没变」不会发生。
fn set_schema_version(tx: &Transaction, version: u32) -> Result<(), String> {
    tx.execute_batch(&format!("PRAGMA user_version = {};", version))
        .map_err(|e| e.to_string())
}
