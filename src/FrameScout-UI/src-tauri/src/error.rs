//! 集中式「错误码 / 状态码」注册表 —— 后端与前端之间的 i18n 契约。
//!
//! ## 为什么要有这个模块
//! 旧实现把**英文整句**直接 `Err(...)` 回传前端，前端只能对少数几串英文做子串
//! 匹配来「翻译」（`useToast.ts::localizeError`），绝大多数报错仍以英文裸露在
//! 中文界面里，且后端一旦改动文案前端立刻失配。
//!
//! 本模块让后端**只回传稳定的码**，文案由前端按码查 i18n，从而真正做到多语言。
//!
//! ## 契约
//! - 传输格式：`CODE` 或 `CODE|param0|param1...`（分隔符 `|`，参数顺序 = 原 `format!` 顺序）
//! - 前端以 `|` 切分：第 1 段为码，其余为插值参数，按码查 i18n 文案。
//! - 文案约定：
//!   - **结构化参数**用 `{0}` `{1}` `{2}`（例如路径、数量）；
//!   - **透传原文**（rusqlite / io / zmq / Python worker 的原生错误）用 `{detail}`
//!     （= 所有参数以 `|` 重新拼接，可避免原文里含 `|` 被切碎）。
//!
//! ## 用法
//! ```ignore
//! return Err(ec(ERR_INDEX_EMPTY));                       // 无参数
//! return Err(ec1(ERR_DB_FILE_NOT_FOUND, path.display())); // 1 个参数
//! .map_err(|e| ec1(ERR_DB_QUERY_FAILED, e))?              // 透传底层错误
//! ```
#![allow(dead_code)]

use std::fmt::Display;

/// 码与参数之间的分隔符。
pub const SEP: char = '|';

/// 无参数的码。
pub fn ec(code: &str) -> String {
    code.to_string()
}

/// 带 1 个参数的码（通常是路径、数量，或透传的底层错误原文）。
pub fn ec1<T: Display>(code: &str, p0: T) -> String {
    format!("{}|{}", code, p0)
}

/// 带 2 个参数的码。
pub fn ec2<T: Display, U: Display>(code: &str, p0: T, p1: U) -> String {
    format!("{}|{}|{}", code, p0, p1)
}

/// 带 3 个参数的码。
pub fn ec3<T: Display, U: Display, V: Display>(code: &str, p0: T, p1: U, p2: V) -> String {
    format!("{}|{}|{}|{}", code, p0, p1, p2)
}

// =========================================================================
//  通用 / 数据库
// =========================================================================

/// 兜底：未知或未分类的错误。
pub const ERR_INTERNAL: &str = "ERR_INTERNAL";
/// `Mutex` / `RwLock` 被污染（另一线程持锁时 panic）。
pub const ERR_LOCK_POISONED: &str = "ERR_LOCK_POISONED";
/// 打开数据库失败。
pub const ERR_DB_OPEN_FAILED: &str = "ERR_DB_OPEN_FAILED";
/// `prepare` 预编译 SQL 失败。
pub const ERR_DB_PREPARE_FAILED: &str = "ERR_DB_PREPARE_FAILED";
/// 查询（`query_map` / `query_row`）失败。
pub const ERR_DB_QUERY_FAILED: &str = "ERR_DB_QUERY_FAILED";
/// `execute` 写操作失败。
pub const ERR_DB_EXEC_FAILED: &str = "ERR_DB_EXEC_FAILED";
/// 计数查询失败。
pub const ERR_DB_COUNT_FAILED: &str = "ERR_DB_COUNT_FAILED";
/// 开启事务失败。
pub const ERR_DB_TX_BEGIN_FAILED: &str = "ERR_DB_TX_BEGIN_FAILED";
/// 提交事务失败。
pub const ERR_DB_TX_COMMIT_FAILED: &str = "ERR_DB_TX_COMMIT_FAILED";
/// 回滚事务失败。
pub const ERR_DB_TX_ROLLBACK_FAILED: &str = "ERR_DB_TX_ROLLBACK_FAILED";
/// `PRAGMA integrity_check` 失败。
pub const ERR_DB_INTEGRITY_FAILED: &str = "ERR_DB_INTEGRITY_FAILED";
/// 读写目录（read_dir / 遍历条目）失败。
pub const ERR_DIR_READ_FAILED: &str = "ERR_DIR_READ_FAILED";
/// 通用文件写失败。
pub const ERR_FILE_WRITE_FAILED: &str = "ERR_FILE_WRITE_FAILED";

// =========================================================================
//  扫描 / 索引
// =========================================================================

/// 用户没有选中任何路径。
pub const ERR_NO_PATHS_SELECTED: &str = "ERR_NO_PATHS_SELECTED";
/// 开启了 OCR 但没指定语言。
pub const ERR_OCR_NO_LANGUAGES: &str = "ERR_OCR_NO_LANGUAGES";
/// 连续失败过多而中止。参数 {0}=连续失败次数。
pub const ERR_SCAN_ABORTED: &str = "ERR_SCAN_ABORTED";
/// 触及免费试用上限。参数 {0}=上限帧数。
pub const ERR_SCAN_TRIAL_LIMIT: &str = "ERR_SCAN_TRIAL_LIMIT";
/// 单个批次索引失败。
pub const ERR_SCAN_BATCH_FAILED: &str = "ERR_SCAN_BATCH_FAILED";
/// 本批索引零帧且存在失败路径（不是「无新文件」，而是「发现但全解码失败」）。参数 {0}=失败路径数。
pub const ERR_SCAN_ALL_FAILED: &str = "ERR_SCAN_ALL_FAILED";

// =========================================================================
//  检索
// =========================================================================

/// 索引库为空（还没扫描过任何文件夹）。
pub const ERR_INDEX_EMPTY: &str = "ERR_INDEX_EMPTY";
/// 提取图片向量失败（文件可能损坏）。
pub const ERR_IMAGE_VECTOR_FAILED: &str = "ERR_IMAGE_VECTOR_FAILED";
/// BGE-M3 文本引擎不可用（模型未下载）。
pub const ERR_BGE_UNAVAILABLE: &str = "ERR_BGE_UNAVAILABLE";

// =========================================================================
//  备份 / 恢复
// =========================================================================

/// 数据库路径未知。
pub const ERR_DB_PATH_UNKNOWN: &str = "ERR_DB_PATH_UNKNOWN";
/// 数据库文件不存在。参数 {0}=路径。
pub const ERR_DB_FILE_NOT_FOUND: &str = "ERR_DB_FILE_NOT_FOUND";
/// 无法确定备份目录。
pub const ERR_BACKUP_DIR_UNKNOWN: &str = "ERR_BACKUP_DIR_UNKNOWN";
/// 创建备份目录失败。
pub const ERR_BACKUP_DIR_CREATE_FAILED: &str = "ERR_BACKUP_DIR_CREATE_FAILED";
/// 备份失败。
pub const ERR_BACKUP_FAILED: &str = "ERR_BACKUP_FAILED";
/// 统计待备份条目数失败。
pub const ERR_BACKUP_COUNT_FAILED: &str = "ERR_BACKUP_COUNT_FAILED";
/// 列出备份文件失败。
pub const ERR_BACKUP_LIST_FAILED: &str = "ERR_BACKUP_LIST_FAILED";
/// 只允许恢复 `.db` 备份文件。
pub const ERR_BACKUP_NOT_DB_FILE: &str = "ERR_BACKUP_NOT_DB_FILE";
/// 拒绝恢复非备份文件（安全护栏）。
pub const ERR_BACKUP_REFUSE_NON_BACKUP: &str = "ERR_BACKUP_REFUSE_NON_BACKUP";
/// 备份文件不存在。参数 {0}=路径。
pub const ERR_BACKUP_FILE_NOT_FOUND: &str = "ERR_BACKUP_FILE_NOT_FOUND";
/// 写入恢复标记文件失败。
pub const ERR_RESTORE_MARKER_FAILED: &str = "ERR_RESTORE_MARKER_FAILED";
/// 只允许删除 `.db` 备份文件。
pub const ERR_BACKUP_DELETE_NOT_DB: &str = "ERR_BACKUP_DELETE_NOT_DB";
/// 删除备份文件失败。
pub const ERR_BACKUP_DELETE_FAILED: &str = "ERR_BACKUP_DELETE_FAILED";

// =========================================================================
//  幽灵清理 / 智能文件夹 / 差异 / 文件
// =========================================================================

/// 未知的幽灵处理动作。参数 {0}=动作名。
pub const ERR_GHOST_UNKNOWN_ACTION: &str = "ERR_GHOST_UNKNOWN_ACTION";
/// 智能文件夹不存在。
pub const ERR_SMART_FOLDER_NOT_FOUND: &str = "ERR_SMART_FOLDER_NOT_FOUND";
/// 还没选择要复查的文件夹。
pub const ERR_DIFF_NO_FOLDER: &str = "ERR_DIFF_NO_FOLDER";
/// 保存笔记时找不到匹配的记录。参数 {0}=路径 {1}=时间戳。
pub const ERR_NOTE_NO_RECORD: &str = "ERR_NOTE_NO_RECORD";

// =========================================================================
//  导出 / ffmpeg
// =========================================================================

/// 导出目录为空。
pub const ERR_EXPORT_DIR_EMPTY: &str = "ERR_EXPORT_DIR_EMPTY";
/// 创建导出目录失败。
pub const ERR_EXPORT_DIR_CREATE_FAILED: &str = "ERR_EXPORT_DIR_CREATE_FAILED";
/// 写出文件失败。
pub const ERR_EXPORT_WRITE_FAILED: &str = "ERR_EXPORT_WRITE_FAILED";
/// 待剪切的源视频不存在。参数 {0}=路径。
pub const ERR_FFMPEG_SOURCE_NOT_FOUND: &str = "ERR_FFMPEG_SOURCE_NOT_FOUND";
/// 调用 ffmpeg 失败（多半是没装 / 不在 PATH 里）。
pub const ERR_FFMPEG_INVOKE_FAILED: &str = "ERR_FFMPEG_INVOKE_FAILED";
/// ffmpeg 剪切失败。参数 {detail}=ffmpeg 的 stderr 前若干字符。
pub const ERR_FFMPEG_CUT_FAILED: &str = "ERR_FFMPEG_CUT_FAILED";

// =========================================================================
//  纯文本通道
// =========================================================================

/// 该格式暂不支持，请先转成 txt/md。参数 {0}=扩展名（大写）。
pub const ERR_TEXT_FORMAT_UNSUPPORTED: &str = "ERR_TEXT_FORMAT_UNSUPPORTED";
/// 不支持的扩展名。参数 {0}=扩展名。
pub const ERR_TEXT_EXT_UNSUPPORTED: &str = "ERR_TEXT_EXT_UNSUPPORTED";
/// 路径不存在。
pub const ERR_TEXT_PATH_NOT_EXIST: &str = "ERR_TEXT_PATH_NOT_EXIST";
/// 读取文件失败。
pub const ERR_TEXT_READ_FAILED: &str = "ERR_TEXT_READ_FAILED";
/// 空文件。
pub const ERR_TEXT_EMPTY_FILE: &str = "ERR_TEXT_EMPTY_FILE";
/// 分块后没有任何文本。
pub const ERR_TEXT_NO_CHUNKS: &str = "ERR_TEXT_NO_CHUNKS";
/// 删除旧分块失败。
pub const ERR_TEXT_DELETE_OLD_CHUNKS: &str = "ERR_TEXT_DELETE_OLD_CHUNKS";
/// 部分成功：参数 {0}=已入库块数，{detail}=失败原因。
pub const ERR_TEXT_PARTIAL_FAILURE: &str = "ERR_TEXT_PARTIAL_FAILURE";

// =========================================================================
//  文本向量存储（storage/text_store.rs）
// =========================================================================

pub const ERR_TEXT_VECTOR_DELETE_FAILED: &str = "ERR_TEXT_VECTOR_DELETE_FAILED";
pub const ERR_TEXT_VECTOR_INSERT_FAILED: &str = "ERR_TEXT_VECTOR_INSERT_FAILED";
/// 稠密向量维度不符。参数 {0}=期望维度 {1}=实际维度。
pub const ERR_TEXT_DENSE_DIM_MISMATCH: &str = "ERR_TEXT_DENSE_DIM_MISMATCH";
pub const ERR_TEXT_ENTRY_INSERT_FAILED: &str = "ERR_TEXT_ENTRY_INSERT_FAILED";
pub const ERR_TEXT_ENTRY_COUNT_FAILED: &str = "ERR_TEXT_ENTRY_COUNT_FAILED";
pub const ERR_TEXT_FTS_PREPARE_FAILED: &str = "ERR_TEXT_FTS_PREPARE_FAILED";
pub const ERR_TEXT_FTS_QUERY_FAILED: &str = "ERR_TEXT_FTS_QUERY_FAILED";
pub const ERR_TEXT_ENTRY_LIST_FAILED: &str = "ERR_TEXT_ENTRY_LIST_FAILED";
pub const ERR_TEXT_ENTRY_QUERY_FAILED: &str = "ERR_TEXT_ENTRY_QUERY_FAILED";
pub const ERR_TEXT_ENTRY_DELETE_FAILED: &str = "ERR_TEXT_ENTRY_DELETE_FAILED";
pub const ERR_TEXT_ENTRY_DELETE_BY_SOURCE_FAILED: &str = "ERR_TEXT_ENTRY_DELETE_BY_SOURCE_FAILED";
pub const ERR_TEXT_ENTRY_COUNT_BY_SOURCE_FAILED: &str = "ERR_TEXT_ENTRY_COUNT_BY_SOURCE_FAILED";

// =========================================================================
//  OCR 存储 / 应用设置
// =========================================================================

pub const ERR_OCR_ENTRIES_DELETE_FAILED: &str = "ERR_OCR_ENTRIES_DELETE_FAILED";
pub const ERR_OCR_ENTRY_INSERT_FAILED: &str = "ERR_OCR_ENTRY_INSERT_FAILED";
pub const ERR_SETTING_SAVE_FAILED: &str = "ERR_SETTING_SAVE_FAILED";

// =========================================================================
//  文件事件 / 批次（storage/journal.rs）
// =========================================================================

pub const ERR_JOURNAL_UPSERT_FILES_FAILED: &str = "ERR_JOURNAL_UPSERT_FILES_FAILED";
pub const ERR_JOURNAL_INSERT_FILES_FAILED: &str = "ERR_JOURNAL_INSERT_FILES_FAILED";
pub const ERR_JOURNAL_SNAPSHOT_FRAMES_FAILED: &str = "ERR_JOURNAL_SNAPSHOT_FRAMES_FAILED";
pub const ERR_JOURNAL_PRUNE_BY_COUNT_FAILED: &str = "ERR_JOURNAL_PRUNE_BY_COUNT_FAILED";
pub const ERR_JOURNAL_PRUNE_BY_AGE_FAILED: &str = "ERR_JOURNAL_PRUNE_BY_AGE_FAILED";
pub const ERR_JOURNAL_RECORD_EVENT_FAILED: &str = "ERR_JOURNAL_RECORD_EVENT_FAILED";
pub const ERR_JOURNAL_BEGIN_BATCH_FAILED: &str = "ERR_JOURNAL_BEGIN_BATCH_FAILED";
pub const ERR_JOURNAL_FINISH_BATCH_FAILED: &str = "ERR_JOURNAL_FINISH_BATCH_FAILED";
pub const ERR_JOURNAL_FINISH_CANCELLED_BATCH_FAILED: &str =
    "ERR_JOURNAL_FINISH_CANCELLED_BATCH_FAILED";
pub const ERR_JOURNAL_MARK_INTERRUPTED_FAILED: &str = "ERR_JOURNAL_MARK_INTERRUPTED_FAILED";

// =========================================================================
//  ZMQ / Python 推理 worker
// =========================================================================

/// worker 返回了无法识别的响应。
pub const ERR_WORKER_UNKNOWN_RESPONSE: &str = "ERR_WORKER_UNKNOWN_RESPONSE";
/// worker 没有返回文本条目。
pub const ERR_WORKER_NO_TEXT_ENTRY: &str = "ERR_WORKER_NO_TEXT_ENTRY";
/// worker 没有返回任何帧。
pub const ERR_WORKER_NO_FRAMES: &str = "ERR_WORKER_NO_FRAMES";
/// worker 侧报错，参数 {detail}=worker 的英文原文。
pub const ERR_WORKER_FAILED: &str = "ERR_WORKER_FAILED";

// =========================================================================
//  OCR 模型管理（commands/model_cmd.rs）
// =========================================================================

/// 语言清单文件不存在。参数 {0}=路径。
pub const ERR_MODEL_MANIFEST_NOT_FOUND: &str = "ERR_MODEL_MANIFEST_NOT_FOUND";
pub const ERR_MODEL_MANIFEST_READ_FAILED: &str = "ERR_MODEL_MANIFEST_READ_FAILED";
pub const ERR_MODEL_MANIFEST_PARSE_FAILED: &str = "ERR_MODEL_MANIFEST_PARSE_FAILED";
/// 未知的语言代码。参数 {0}=语言代码。
pub const ERR_MODEL_UNKNOWN_LANGUAGE: &str = "ERR_MODEL_UNKNOWN_LANGUAGE";
/// 打开模型文件失败。参数 {0}=路径。
pub const ERR_MODEL_FILE_OPEN_FAILED: &str = "ERR_MODEL_FILE_OPEN_FAILED";
pub const ERR_MODEL_FILE_READ_FAILED: &str = "ERR_MODEL_FILE_READ_FAILED";
/// 磁盘空间不足。参数 {0}=需要字节 {1}=可用字节。
pub const ERR_MODEL_DISK_SPACE: &str = "ERR_MODEL_DISK_SPACE";
pub const ERR_MODEL_DOWNLOAD_FAILED: &str = "ERR_MODEL_DOWNLOAD_FAILED";
/// 创建分片文件失败。参数 {0}=路径。
pub const ERR_MODEL_PART_CREATE_FAILED: &str = "ERR_MODEL_PART_CREATE_FAILED";
pub const ERR_MODEL_CHUNK_FAILED: &str = "ERR_MODEL_CHUNK_FAILED";
pub const ERR_MODEL_WRITE_FAILED: &str = "ERR_MODEL_WRITE_FAILED";
/// SHA256 校验不符。参数 {0}=期望值 {1}=实际值。
pub const ERR_MODEL_SHA_MISMATCH: &str = "ERR_MODEL_SHA_MISMATCH";
pub const ERR_MODEL_FINALIZE_FAILED: &str = "ERR_MODEL_FINALIZE_FAILED";
pub const ERR_MODEL_NO_URLS: &str = "ERR_MODEL_NO_URLS";
/// 该模型计划支持但尚未提供。参数 {0}=语言代码。
pub const ERR_MODEL_NOT_AVAILABLE: &str = "ERR_MODEL_NOT_AVAILABLE";
pub const ERR_MODEL_DIR_CREATE_FAILED: &str = "ERR_MODEL_DIR_CREATE_FAILED";
/// 内置模型不可删除。参数 {0}=语言代码。
pub const ERR_MODEL_BUILTIN_NO_DELETE: &str = "ERR_MODEL_BUILTIN_NO_DELETE";
pub const ERR_MODEL_DELETE_FAILED: &str = "ERR_MODEL_DELETE_FAILED";

// =========================================================================
//  授权（license/）
// =========================================================================

pub const ERR_LICENSE_PUBKEY_HEX_FAILED: &str = "ERR_LICENSE_PUBKEY_HEX_FAILED";
pub const ERR_LICENSE_PUBKEY_PARSE_FAILED: &str = "ERR_LICENSE_PUBKEY_PARSE_FAILED";
pub const ERR_LICENSE_BASE64_FAILED: &str = "ERR_LICENSE_BASE64_FAILED";
pub const ERR_LICENSE_KEY_TOO_SHORT: &str = "ERR_LICENSE_KEY_TOO_SHORT";
pub const ERR_LICENSE_KEY_MALFORMED: &str = "ERR_LICENSE_KEY_MALFORMED";
/// 签名长度不符。参数 {0}=实际长度。
pub const ERR_LICENSE_SIG_LENGTH: &str = "ERR_LICENSE_SIG_LENGTH";
pub const ERR_LICENSE_SIG_BYTES_FAILED: &str = "ERR_LICENSE_SIG_BYTES_FAILED";
pub const ERR_LICENSE_SIG_VERIFY_FAILED: &str = "ERR_LICENSE_SIG_VERIFY_FAILED";
pub const ERR_LICENSE_PAYLOAD_NOT_UTF8: &str = "ERR_LICENSE_PAYLOAD_NOT_UTF8";
pub const ERR_LICENSE_EMAIL_MISMATCH: &str = "ERR_LICENSE_EMAIL_MISMATCH";
pub const ERR_LICENSE_PRODUCT_MISMATCH: &str = "ERR_LICENSE_PRODUCT_MISMATCH";
/// 版本锁定不符。参数 {0}=授权锁定版本 {1}=当前构建版本。
pub const ERR_LICENSE_VERSION_LOCKED: &str = "ERR_LICENSE_VERSION_LOCKED";
pub const ERR_LICENSE_EXPIRY_INVALID: &str = "ERR_LICENSE_EXPIRY_INVALID";
pub const ERR_LICENSE_CLOCK_ERROR: &str = "ERR_LICENSE_CLOCK_ERROR";
pub const ERR_LICENSE_EXPIRED: &str = "ERR_LICENSE_EXPIRED";
/// payload 字段数不符。参数 {0}=实际字段数。
pub const ERR_LICENSE_PAYLOAD_FIELDS: &str = "ERR_LICENSE_PAYLOAD_FIELDS";
pub const ERR_LICENSE_WRITE_FAILED: &str = "ERR_LICENSE_WRITE_FAILED";
/// 开源版不提供 Pro 激活。
pub const ERR_LICENSE_EDITION_UNAVAILABLE: &str = "ERR_LICENSE_EDITION_UNAVAILABLE";
/// 激活成功。参数 {0}=邮箱。
pub const MSG_LICENSE_ACTIVATED: &str = "MSG_LICENSE_ACTIVATED";

// =========================================================================
//  状态文案（emit 事件 payload，非错误）
// =========================================================================

pub const STATUS_SCAN_DONE: &str = "STATUS_SCAN_DONE";
pub const STATUS_SCAN_NO_NEW_FILES: &str = "STATUS_SCAN_NO_NEW_FILES";
pub const STATUS_SCAN_PAUSED: &str = "STATUS_SCAN_PAUSED";
pub const STATUS_SCAN_RESUMING: &str = "STATUS_SCAN_RESUMING";
pub const STATUS_SCAN_CANCELLED: &str = "STATUS_SCAN_CANCELLED";
pub const STATUS_SCAN_ENGINE_RESET: &str = "STATUS_SCAN_ENGINE_RESET";
/// 暂停时的补充说明行（`file_path` 字段承载）。
pub const STATUS_SCAN_PAUSED_NOTE: &str = "STATUS_SCAN_PAUSED_NOTE";
/// 继续时的补充说明行。
pub const STATUS_SCAN_RESUMED_NOTE: &str = "STATUS_SCAN_RESUMED_NOTE";
/// 取消时的补充说明行（提示部分结果已保存）。
pub const STATUS_SCAN_CANCELLED_NOTE: &str = "STATUS_SCAN_CANCELLED_NOTE";
/// 重置引擎时的补充说明行（提示正在释放 GPU 资源）。
pub const STATUS_SCAN_ENGINE_RESET_NOTE: &str = "STATUS_SCAN_ENGINE_RESET_NOTE";
pub const STATUS_SCAN_TRIAL_LIMIT: &str = "STATUS_SCAN_TRIAL_LIMIT";
/// 批处理中。参数 {0}=当前序号 {1}=总数 {2}=本组数量。
pub const STATUS_SCAN_BATCH_PROCESSING: &str = "STATUS_SCAN_BATCH_PROCESSING";
pub const STATUS_SCAN_BATCH_FAILED: &str = "STATUS_SCAN_BATCH_FAILED";
pub const STATUS_SCAN_ABORTED: &str = "STATUS_SCAN_ABORTED";
pub const STATUS_SCAN_QUEUE_FINISHED: &str = "STATUS_SCAN_QUEUE_FINISHED";
pub const STATUS_SCAN_RAPID_PRESCAN: &str = "STATUS_SCAN_RAPID_PRESCAN";
pub const STATUS_SCAN_NO_NEW_FOUND: &str = "STATUS_SCAN_NO_NEW_FOUND";
/// 正在索引 N 个文件。参数 {0}=文件数。
pub const STATUS_SCAN_INDEXING: &str = "STATUS_SCAN_INDEXING";
pub const STATUS_SCAN_ALL_INDEXED: &str = "STATUS_SCAN_ALL_INDEXED";

pub const STATUS_ENGINE_READY: &str = "STATUS_ENGINE_READY";
/// 正在连接引擎。参数 {0}=当前重试次数 {1}=最大重试次数。
pub const STATUS_ENGINE_CONNECTING: &str = "STATUS_ENGINE_CONNECTING";
pub const STATUS_ENGINE_FAILED: &str = "STATUS_ENGINE_FAILED";

// =========================================================================
//  匹配来源徽标（matched_tags，由前端直接渲染）
// =========================================================================

pub const TAG_VISUAL: &str = "TAG_VISUAL";
pub const TAG_OCR: &str = "TAG_OCR";
pub const TAG_NOTE: &str = "TAG_NOTE";
pub const TAG_FILENAME: &str = "TAG_FILENAME";
pub const TAG_UNKNOWN: &str = "TAG_UNKNOWN";
pub const TAG_ALL_FILES: &str = "TAG_ALL_FILES";
