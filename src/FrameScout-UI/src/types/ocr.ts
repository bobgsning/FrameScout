/** OCR 语言分层：内置（wheel）/ 内置（随包）/ 可下载 / 规划占位 */
export type OcrTier = 'builtin_wheel' | 'builtin_bundle' | 'downloadable' | 'planned'

/** 后端 `list_ocr_languages` 返回的单个语言状态 */
export interface OcrLanguageInfo {
  code: string
  native: string
  tier: OcrTier
  deletable: boolean
  /** 该语言的识别模型当前是否可用（内置恒可用；随包/可下载需文件存在） */
  available: boolean
}

/** OCR 模型下载进度（来自 Rust 侧事件） */
export interface OcrDownloadProgress {
  language: string
  downloaded: number
  total: number
  percent: number
}
