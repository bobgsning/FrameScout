/**
 * License & engine lifecycle types
 * Migrated from the original App.vue <script setup> declarations.
 */

export interface LicenseStatus {
  is_pro: boolean
  email: string
  limit: number
  /** 到期时间戳（Unix 秒）；旧格式 Key 或未激活时为 null */
  expiry?: number | null
  /** 锁定的版本号；旧格式 Key 或未激活时为 null */
  version?: string | null
}

export type EngineStatusValue = 'connecting' | 'ready' | 'error'

/** Rust 端 engine-status 事件负载 */
export interface EngineStatusPayload {
  status: string
  retry?: number
  max_retries?: number
  message: string
}

/** Rust 端 scan-progress 事件负载 */
export interface ScanProgressPayload {
  status: string
  file_path: string
  current: number
  total: number
  new_files?: string[]
}
