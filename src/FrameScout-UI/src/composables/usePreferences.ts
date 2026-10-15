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
const KEY_OCR_LANG = 'framescout_ocr_lang'          // 旧：逗号分隔字符串（迁移用）
const KEY_OCR_LANGS = 'framescout_ocr_languages'    // 新：JSON 数组
const KEY_SCAN_MODE = 'framescout_scan_mode'
const KEY_SEARCH_HISTORY = 'framescout_search_history'
const KEY_SEARCH_HISTORY_LIMIT = 'framescout_search_history_limit'
const KEY_RELEVANCE_SENSITIVITY = 'framescout_relevance_sensitivity'
const KEY_LOW_SCORE_FOLD = 'framescout_low_score_fold'
const KEY_ENABLE_OCR = 'framescout_enable_ocr'
const KEY_LANGUAGE = 'framescout_language'
const KEY_EXPORT_PATH = 'framescout_export_path'

// ---------- 模块级共享状态 ----------
const folderPath = ref(loadString(KEY_FOLDER_PATH, ''))
const pageSize = ref(loadNumber(KEY_PAGE_SIZE, 8))
const ocrLanguages = ref<string[]>(loadOcrLanguages())
const scanMode = ref(loadString(KEY_SCAN_MODE, 'all'))
const searchHistory = ref<string[]>(loadArray(KEY_SEARCH_HISTORY, []))
// M0'''：搜索历史保留条数（默认 20，-1 = 不限）。
const searchHistoryLimit = ref(loadNumber(KEY_SEARCH_HISTORY_LIMIT, 20))
// M0'''：相关度灵敏度（loose / standard / strict），默认 standard。
const relevanceSensitivity = ref<'loose' | 'standard' | 'strict'>(loadSensitivity())
// M0'''：低分自动折叠开关（默认开，可关——「不折叠」是一等选项）。
const lowScoreFoldEnabled = ref(loadBool(KEY_LOW_SCORE_FOLD, true))
// M0'''：OCR 开关（此前每次重启静默回到 false，现持久化）。
const enableOcr = ref(loadBool(KEY_ENABLE_OCR, false))
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

function loadBool(key: string, fallback: boolean): boolean {
  try {
    const v = localStorage.getItem(key)
    if (v == null) return fallback
    return v === 'true' || v === '1'
  } catch {
    return fallback
  }
}

function loadSensitivity(): 'loose' | 'standard' | 'strict' {
  const v = loadString(KEY_RELEVANCE_SENSITIVITY, 'standard')
  return v === 'loose' || v === 'strict' ? v : 'standard'
}

/** OCR 语言偏好：旧版存逗号分隔字符串，新版存 JSON 数组，此处做一次迁移。 */
function loadOcrLanguages(): string[] {
  try {
    const legacy = localStorage.getItem(KEY_OCR_LANG)
    if (legacy && !legacy.trimStart().startsWith('[')) {
      localStorage.removeItem(KEY_OCR_LANG)
      const arr = legacy.split(',').map((s) => s.trim()).filter(Boolean)
      if (arr.length) return arr
    }
  } catch {
    // ignore
  }
  const arr = loadArray(KEY_OCR_LANGS, ['en'])
  const filtered = arr.filter((x): x is string => typeof x === 'string')
  return filtered.length ? filtered : ['en']
}

// ---------- 自动持久化 ----------
watch(folderPath, (v) => save(KEY_FOLDER_PATH, v))
watch(pageSize, (v) => save(KEY_PAGE_SIZE, String(v)))
watch(ocrLanguages, (v) => save(KEY_OCR_LANGS, JSON.stringify(v)), { deep: true })
watch(scanMode, (v) => save(KEY_SCAN_MODE, v))
watch(searchHistory, (v) => save(KEY_SEARCH_HISTORY, JSON.stringify(v)), { deep: true })
watch(searchHistoryLimit, (v) => save(KEY_SEARCH_HISTORY_LIMIT, String(v)))
watch(relevanceSensitivity, (v) => save(KEY_RELEVANCE_SENSITIVITY, v))
watch(lowScoreFoldEnabled, (v) => save(KEY_LOW_SCORE_FOLD, String(v)))
watch(enableOcr, (v) => save(KEY_ENABLE_OCR, String(v)))
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
// M0'''：历史保留条数改为用户可设（默认 20，-1 = 不限），不再静默截断。

function addSearchHistory(query: string): void {
  const trimmed = query.trim()
  if (!trimmed) return
  // 去重：已存在则提到最前
  const idx = searchHistory.value.indexOf(trimmed)
  if (idx >= 0) searchHistory.value.splice(idx, 1)
  searchHistory.value.unshift(trimmed)
  // -1 = 不限：不截断
  const limit = searchHistoryLimit.value
  if (limit >= 0 && searchHistory.value.length > limit) {
    searchHistory.value.length = limit
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
    ocrLanguages,
    scanMode,
    searchHistory,
    searchHistoryLimit,
    relevanceSensitivity,
    lowScoreFoldEnabled,
    enableOcr,
    language,
    exportPath,
    addSearchHistory,
    clearSearchHistory,
    removeSearchHistoryEntry,
    suggest,
  }
}
