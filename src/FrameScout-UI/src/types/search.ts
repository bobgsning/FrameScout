/**
 * Search & media domain types
 * Migrated from the original App.vue <script setup> interface declarations.
 */

/** 一条 OCR 命中行（P1-2）：含归一化 bbox 供前端画红框高亮。 */
export interface OcrLineHit {
  text: string
  bbox_left: number
  bbox_top: number
  bbox_right: number
  bbox_bottom: number
  conf: number
  lang: string
  /** 帧时间戳（秒） */
  timestamp: number
}

/** 单个通道的命中来源（P2-6 / search_unified）：供前端画「得分来源条」。 */
export interface ChannelMatch {
  /** visual | ocr | note | filename */
  channel: string
  /** 该通道的归一化分数（0~1） */
  score: number
}

export interface SearchResult {
  path: string
  timestamp: number
  score: number
  matched_tags: string[]
  ocr_text: string
  user_note: string
  /** 入库时间（Unix 秒）；后端返回但旧类型未声明，补齐 */
  index_time?: number

  /** 命中的 OCR 行级数据（P1-2）：含 bbox 供前端画红框 */
  ocr_lines?: OcrLineHit[]
  /** 命中处数（P1-2：「N 处匹配」徽标） */
  match_count?: number
  /** 各通道命中来源（P2-6）：search_unified 返回，供前端画得分来源条 */
  matches?: ChannelMatch[]

  /** 原始 SigLIP 视觉余弦（方案 C：不 sigmoid、不加权，0~1）；非视觉通道恒为 0 */
  visual_similarity?: number
  /** 各字面通道命中详情（方案 C）：供前端自决融合与得分来源条 */
  ocr_hit?: boolean
  note_hit?: boolean
  filename_hit?: boolean

  /** 文件在磁盘上已不存在（视频加载失败 / 25s 超时后由前端标记） */
  isMissing?: boolean
  /** OCR 文本是否展开（超过 50 字时可折叠） */
  expandOcr?: boolean
  /** 低置信度图片是否折叠（< 30% 时默认收起） */
  collapsedLowScore?: boolean
  ocrRunning?: boolean
  ocrLangInput?: string
  /** 分数来源：决定前端用哪套映射（SigLIP 视觉 / BGE-M3 文本 / 以图搜图） */
  scoreKind?: ScoreKind
}

/**
 * 搜索模式：决定「怎么匹配」。
 *  - exact    ：字符串精确匹配（文件名 / 笔记 / OCR 的 contains）
 *  - semantic ：SigLIP 视觉语义（以文搜图，找画面）
 *  - bge      ：BGE-M3 文本语义（OCR 文本的 dense 检索）
 * 三种模式是三条检索公路，前端按用户选择映射，后端各给各的原始分数。
 *
 * ⚠️ 第七轮起废弃：逻辑搜索重构后，模式由「条件行」表达，此类型仅保留向后兼容。
 */
export type SearchMode = 'exact' | 'semantic' | 'bge'

/**
 * 第七轮 逻辑搜索重构：条件行类型。
 *  - text         ：文字（字面 contains），作用域 targets 决定在哪些通道匹配
 *  - visual       ：画面（SigLIP 视觉语义，以文搜图）
 *  - text_semantic：文本语义（BGE-M3 dense，实验性，互斥）
 *  - text_entry   ：纯文本条目（BGE-M3，独立结果集，互斥）
 *  - time         ：入库时间范围（L1 后端过滤）
 *  - path         ：目录范围（L1 后端过滤）
 *  - batch        ：来源批次（L1 后端过滤）
 *  - file_type    ：文件类型（图片/视频/文本，L1 后端过滤）
 *  - seed         ：基准图（以图搜图）
 *  - exclude      ：排除（NOT）
 */
export type ConditionType =
  | 'text' | 'visual' | 'text_semantic' | 'text_entry'
  | 'time' | 'path' | 'batch' | 'file_type' | 'seed' | 'exclude'

/** 文字/排除条件的作用域：在哪些通道匹配 */
export type ConditionTarget = 'filename' | 'note' | 'ocr'

/** 组级逻辑：全部满足（AND）/ 任一满足（OR）。不提供嵌套。 */
export type GroupOp = 'all' | 'any'

/** 一条搜索条件（第七轮逻辑搜索重构的核心数据模型，取代 searchMode + s_* 布尔）。 */
export interface ConditionRow {
  /** 前端生成的唯一 id（用于 v-for key 与「按 ① 排序」的序号引用） */
  id: string
  type: ConditionType
  /** 文字/排除/文本语义/基准图的值 */
  value?: string
  /** text / exclude 的作用域；空 = 全部文字通道 */
  targets?: ConditionTarget[]
  /** visual / seed 的最低人类分阈值（0-100，null = 不设） */
  minScore?: number | null
  /** time：入库起始（Unix 秒） */
  timeFrom?: number | null
  /** time：入库截止（Unix 秒） */
  timeTo?: number | null
  /** file_type：文件类型 */
  fileTypes?: string[]
  /** path：目录前缀 */
  pathPrefix?: string
  /** batch：来源批次 id */
  batchId?: string
  /** seed：基准图路径 */
  seedPath?: string
}

/**
 * 分数来源（量纲）。后端返回的是各自通道的原始相似度，量纲不同：
 *  - siglip ：SigLIP 视觉 cosine（以文搜图 / 混合检索的语义部分）
 *  - bge    ：BGE-M3 dense cosine（文本语义，量纲远高于 SigLIP 视觉 cosine）
 *  - image  ：以图搜图的视觉 cosine
 * 前端据此选正确的映射，避免跨量纲比较。
 */
export type ScoreKind = 'siglip' | 'bge' | 'image'

/**
 * BGE-M3 文本语义检索的命中（第一手原始数据）。
 * `similarity` 是未加权的 dense 余弦——是否与视觉 / 精确匹配混合、怎么加权，
 * 由前端映射，后端不替用户做决定。
 */
export interface TextSearchHit {
  path: string
  /** 视频帧时间戳（秒）；图片恒为 0 */
  timestamp: number
  /** BGE-M3 dense 余弦相似度（原始、未加工） */
  similarity: number
  ocr_text: string
  user_note: string
  index_time: number
}

/**
 * 纯文本条目（`text_entries`）检索的命中（第一手原始数据）。
 * 与媒体命中不同：纯文本是独立检索对象，命中展示的是内容本身而非图片/视频帧。
 */
export interface TextEntryHit {
  entry_id: string
  source_uri: string
  content: string
  /** BGE-M3 dense 余弦相似度（原始、未加工） */
  similarity: number
  index_time: number
  /** 长文本是否展开（前端折叠态；与图片卡片的 expandOcr 同理） */
  expand?: boolean
}

/** 纯文本条目的一条记录（管理视图：列出库里已有哪些条目） */
export interface TextEntryItem {
  entry_id: string
  source_uri: string
  content: string
  index_time: number
  /** 承载分块信息（chunk_index / chunk_count）等扩展数据 */
  metadata_json: string | null
}

/** 纯文本条目列表的分页响应 */
export interface TextEntryListResponse {
  items: TextEntryItem[]
  total_count: number
}

/** 文本文件入库失败的明细 */
export interface TextIngestFailure {
  path: string
  error: string
}

/** 文本文件批量入库的结果（长文档会切成多块，故 chunks 通常多于文件数） */
export interface TextIngestResult {
  ingested_files: string[]
  chunks: number
  failed: TextIngestFailure[]
  /** M0''' 告知：重新导入时被替换掉的旧块数 */
  deleted_old_chunks: number
}

/** 入库前预演（删前预告的准确数字，非全库总数）。 */
export interface TextIngestPreview {
  /** 将被替换掉的旧块总数 */
  will_replace_chunks: number
  /** 会动到的来源文件数 */
  affected_files: number
}

export interface ClusterGroup {
  group_id: number
  /** 簇代表帧（medoid）：簇内与其它帧平均相似度最高的成员 */
  representative_path: string
  member_paths: string[]
}

/** 聚类结果（债单 B8：告知被截断的独特帧数） */
export interface ClusterResult {
  groups: ClusterGroup[]
  truncated_single_frames: number
}

export interface SmartFolder {
  id: number
  name: string
  query_text: string
  use_vector: boolean
  use_ocr: boolean
  use_note: boolean
  use_filename: boolean
  /** 后端动态返回的匹配数；use_vector 为 true 且值为 0 时前端显示 "?" */
  match_count: number
  /** 第八轮 M011：完整条件行的序列化（`ConditionRow[]`）。null = 旧文件夹，按上面 7 字段解释。 */
  criteria_json?: string | null
  /** true = 这个文件夹必须由前端条件引擎求值，列表里的数字不是它的匹配数（显示 "?"）。 */
  needs_frontend_eval?: boolean
}

/** 智能文件夹导入的后端返回：如实报告「导入了几条、其中几条因重名改了名」。 */
export interface SmartFolderImportResult {
  imported: number
  renamed: number
  names: string[]
}

/** list_all_files / search_by_image 的统一分页响应 */
export interface PagedResponse<T> {
  items: T[]
  total_count: number
}

export interface ScanProgress {
  current: number
  total: number
}

/** 幽灵清理预览项：系统只负责呈现事实，怎么处理由用户决定 */
export interface GhostItem {
  path: string
  /** 仍存的向量行数（视频 = 帧数），让用户理解「真删除会失去什么」 */
  frame_count: number
  observed_size: number | null
  observed_mtime: number | null
  /** 系统最后一次确认该文件存在的时刻 */
  last_seen_at: string | null
  /** 被标记失效的时刻（未失效则为 null） */
  dead_at: string | null
  /** 已标记失效：占着库，但不参与检索，随时可回库 */
  is_dead: boolean
  /** 此刻磁盘上是否存在 */
  exists_on_disk: boolean
  /** 该文件所在盘根当前不可达（P0-1）：NAS 未挂载 / 移动硬盘未插。
   *  true 时前端整组灰显并禁用 purge——避免误删只是暂时离线的整个盘。 */
  disk_offline: boolean
}

/** 差异报告的四组分类 */
export type DiffKind = 'new' | 'modified' | 'missing' | 'unindexed'

export interface DiffItem {
  path: string
  kind: DiffKind
  /** 库中记录的观察值（新增项为 null） */
  observed_mtime: number | null
  observed_size: number | null
  /** 磁盘当前值（消失项为 null） */
  disk_mtime: number | null
  disk_size: number | null
  frame_count: number
  last_seen_at: string | null
}

export interface DiffReport {
  /** 骨架：最近一次合法完成的扫描批次 */
  baseline_batch_id: string | null
  root: string
  scanned_files: number
  /** 疑似外部同步导致的大量变化 —— 只提示，由用户判断 */
  mass_change_suspected: boolean
  new_count: number
  modified_count: number
  missing_count: number
  unindexed_count: number
  items: DiffItem[]
}

/** batches 表的一条记录：骨架 + 摘要 */
export interface BatchReport {
  id: number
  batch_id: string
  /** scan | clean_ghosts | reindex | legacy_import */
  batch_type: string
  started_at: string | null
  /** 非空 = 合法完成 */
  finished_at: string | null
  /** running | completed | interrupted */
  status: string
  summary_json: string | null
}

export interface ReindexFailure {
  path: string
  error: string
}

export interface ReindexResult {
  batch_id: string
  succeeded: string[]
  /** 失败的路径已写入 reindex_failed 事件，仍留在待索引组 */
  failed: ReindexFailure[]
  replaced_frames: number
  new_frames: number
}

export interface GhostActionResult {
  batch_id: string
  /** mark_dead | restore | purge */
  action: string
  affected_paths: string[]
  affected_frames: number
  /** 状态不符而被跳过的路径 */
  skipped_paths: string[]
}

/** M0''' 三档自动清理策略（与后端 settings_cmd.rs::CleanupSettings 对应）。
 *  `-1` = 「永不 / 不限」。 */
export interface CleanupSettings {
  event_retention_days: number
  backup_keep_count: number
  backup_max_age_days: number
}

/** M0''' 启动期自动清理汇总（供前端 Toast 告知「系统删了什么」） */
export interface StartupCleanupReport {
  pruned_events: number
  pruned_batches: number
  pruned_reindex_backups: number
  cleaned_residual_rows: number
  /** 第八轮：这次清理的时间（UTC ISO）。未发生清理时为 null。 */
  cleaned_at?: string | null
}

/** 立即备份的返回（M0'''：含被轮转删除的旧备份文件名） */
export interface BackupResult {
  path: string
  removed: string[]
}
