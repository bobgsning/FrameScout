/**
 * usePreferences — 用户偏好持久化（P1-5 / 第三轮「上次目录每次被主动清除」）。
 *
 * 旧实现：useEngineStatus 每次 onMounted 都 `localStorage.removeItem('framescout_folder_path')`，
 * 导致每天打开 App 第一件事是重新粘贴素材库路径。
 * 新实现：把 folder_path / page_size / ocr_lang 等持久化到 localStorage，
 * 重开 App 自动回填。仅显式「清除」按钮才会删。
 */
import { ref, watch } from 'vue'

// ---------- 持久化键 ----------
const KEY_FOLDER_PATH = 'framescout_folder_path'
const KEY_PAGE_SIZE = 'framescout_page_size'
const KEY_OCR_LANG = 'framescout_ocr_lang'
const KEY_SCAN_MODE = 'framescout_scan_mode'
const KEY_SEARCH_HISTORY = 'framescout_search_history'
const KEY_LANGUAGE = 'framescout_language'
const KEY_EXPORT_PATH = 'framescout_export_path'

// ---------- 模块级共享状态 ----------
const folderPath = ref(loadString(KEY_FOLDER_PATH, ''))
const pageSize = ref(loadNumber(KEY_PAGE_SIZE, 8))
const ocrLang = ref(loadString(KEY_OCR_LANG, 'en'))
const scanMode = ref(loadString(KEY_SCAN_MODE, 'all'))
const searchHistory = ref<string[]>(loadArray(KEY_SEARCH_HISTORY, []))
// UI 语言：默认英文，中文为第二语言
const language = ref(loadString(KEY_LANGUAGE, 'en'))
// 导出目录（空 = 回退浏览器下载）
const exportPath = ref(loadString(KEY_EXPORT_PATH, ''))

function loadString(key: string, fallback: string): string {
  try {
    const v = localStorage.getItem(key)
    return v ?? fallback
  } catch {
    return fallback
  }
}

function loadNumber(key: string, fallback: number): number {
  try {
    const v = localStorage.getItem(key)
    return v ? Number(v) || fallback : fallback
  } catch {
    return fallback
  }
}

function loadArray(key: string, fallback: string[]): string[] {
  try {
    const v = localStorage.getItem(key)
    return v ? JSON.parse(v) : fallback
  } catch {
    return fallback
  }
}

// ---------- 自动持久化 ----------
watch(folderPath, (v) => save(KEY_FOLDER_PATH, v))
watch(pageSize, (v) => save(KEY_PAGE_SIZE, String(v)))
watch(ocrLang, (v) => save(KEY_OCR_LANG, v))
watch(scanMode, (v) => save(KEY_SCAN_MODE, v))
watch(searchHistory, (v) => save(KEY_SEARCH_HISTORY, JSON.stringify(v)), { deep: true })
watch(language, (v) => save(KEY_LANGUAGE, v))
watch(exportPath, (v) => save(KEY_EXPORT_PATH, v))

function save(key: string, value: string) {
  try {
    localStorage.setItem(key, value)
  } catch {
    // localStorage 满或被禁用时静默失败
  }
}

// ---------- 搜索历史 ----------
const MAX_HISTORY = 20

function addSearchHistory(query: string): void {
  const trimmed = query.trim()
  if (!trimmed) return
  // 去重：已存在则提到最前
  const idx = searchHistory.value.indexOf(trimmed)
  if (idx >= 0) searchHistory.value.splice(idx, 1)
  searchHistory.value.unshift(trimmed)
  if (searchHistory.value.length > MAX_HISTORY) {
    searchHistory.value.length = MAX_HISTORY
  }
}

function clearSearchHistory(): void {
  searchHistory.value = []
}

function removeSearchHistoryEntry(query: string): void {
  const idx = searchHistory.value.indexOf(query)
  if (idx >= 0) searchHistory.value.splice(idx, 1)
}

// ---------- 联想：前缀匹配 ----------
function suggest(prefix: string, limit = 5): string[] {
  const p = prefix.trim().toLowerCase()
  if (!p) return []
  return searchHistory.value
    .filter((q) => q.toLowerCase().startsWith(p))
    .slice(0, limit)
}

export function usePreferences() {
  return {
    folderPath,
    pageSize,
    ocrLang,
    scanMode,
    searchHistory,
    language,
    exportPath,
    addSearchHistory,
    clearSearchHistory,
    removeSearchHistoryEntry,
    suggest,
  }
}
