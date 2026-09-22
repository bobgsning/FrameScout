/**
 * License & engine lifecycle types
 * Migrated from the original App.vue <script setup> declarations.
 */

export interface LicenseStatus {
  is_pro: boolean
  email: string
  limit: number
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

export interface CleanGhostsResult {
  removed_count: number
  removed_paths: string[]
}
