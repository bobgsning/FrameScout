/**
 * Search & media domain types
 * Migrated from the original App.vue <script setup> interface declarations.
 */

export interface SearchResult {
  path: string
  timestamp: number
  score: number
  matched_tags: string[]
  ocr_text: string
  user_note: string

  /** 文件在磁盘上已不存在（视频加载失败 / 25s 超时后由前端标记） */
  isMissing?: boolean
  /** OCR 文本是否展开（超过 50 字时可折叠） */
  expandOcr?: boolean
  /** 低置信度图片是否折叠（< 30% 时默认收起） */
  collapsedLowScore?: boolean
  ocrRunning?: boolean
  ocrLangInput?: string
}

export interface ClusterGroup {
  group_id: number
  member_paths: string[]
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
}

/** list_all_files / search_images / search_by_image / execute_smart_folder 的统一分页响应 */
export interface PagedResponse<T> {
  items: T[]
  total_count: number
}

export interface ScanProgress {
  current: number
  total: number
}
