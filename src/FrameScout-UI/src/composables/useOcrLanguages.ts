/**
 * useOcrLanguages — OCR 多语言列表状态管理。
 *
 * 从后端 `list_ocr_languages` 拉取语言目录（内置 / 可下载 / 规划三档），
 * 与 `usePreferences.ocrLanguages`（已选语言数组）联动；提供下载（含进度）、
 * 校验、删除模型动作。下载进度通过 `ocr-download-progress` 事件上报。
 */
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { OcrLanguageInfo, OcrDownloadProgress } from '@/types/ocr'
import { usePreferences } from './usePreferences'
import { useToast } from './useToast'
import { t } from '@/i18n'

// 模块级共享状态：设置页打开即拉取一次，多语言列表跨组件复用
const languages = ref<OcrLanguageInfo[]>([])
const loading = ref(false)
/** 正在下载的语言码（一次只允许一个） */
const downloading = ref<string | null>(null)
/** 语言码 → 下载进度 */
const progress = ref<Record<string, OcrDownloadProgress>>({})
/** 最近一次下载失败的语言码（用于按钮显示「重试」） */
const failed = ref<string[]>([])

const { ocrLanguages: selected } = usePreferences()

/** 已选语言集合（用于勾选态渲染） */
const selectedSet = computed(() => new Set(selected.value))

/** 三档分组 */
const builtinLanguages = computed(() =>
  languages.value.filter((l) => l.tier === 'builtin_wheel' || l.tier === 'builtin_bundle')
)
const downloadableLanguages = computed(() =>
  languages.value.filter((l) => l.tier === 'downloadable')
)
const plannedLanguages = computed(() =>
  languages.value.filter((l) => l.tier === 'planned')
)

function isSelected(code: string): boolean {
  return selectedSet.value.has(code)
}

function toggle(code: string): void {
  const idx = selected.value.indexOf(code)
  if (idx >= 0) {
    selected.value.splice(idx, 1)
  } else {
    // 识别策略互斥：en_opt（纯英文模型）与 en/ch_sim（中英合并模型）不能同时开启，
    // 否则英文文本会被两种模型重复识别。勾选一方自动取消另一方。
    if (code === 'en_opt') {
      selected.value = selected.value.filter((c) => c !== 'en' && c !== 'ch_sim')
    } else if (code === 'en' || code === 'ch_sim') {
      selected.value = selected.value.filter((c) => c !== 'en_opt')
    }
    selected.value.push(code)
  }
}

async function refresh(): Promise<void> {
  loading.value = true
  try {
    languages.value = await invoke<OcrLanguageInfo[]>('list_ocr_languages')
  } catch (err) {
    languages.value = []
    const { push, localizeError } = useToast()
    push(localizeError(err, t('ocr.verifyFailed')), 'error')
  } finally {
    loading.value = false
  }
}

// 进度事件只订阅一次
let progressBound = false
function bindProgressEvents(): void {
  if (progressBound) return
  progressBound = true
  listen<OcrDownloadProgress>('ocr-download-progress', (e) => {
    const p = e.payload
    progress.value = { ...progress.value, [p.language]: p }
  })
}

async function downloadModel(code: string): Promise<void> {
  bindProgressEvents()
  const { push, localizeError } = useToast()
  if (downloading.value) return

  downloading.value = code
  failed.value = failed.value.filter((c) => c !== code)
  progress.value = { ...progress.value, [code]: { language: code, downloaded: 0, total: 0, percent: 0 } }
  try {
    await invoke('download_ocr_model', { language: code })
    const lang = languages.value.find((l) => l.code === code)
    push(t('ocr.downloadComplete', { name: lang?.native || code }), 'success')
    await refresh()
  } catch (err) {
    failed.value = [...failed.value.filter((c) => c !== code), code]
    push(localizeError(err, t('ocr.downloadFailed', { err: String(err) })), 'error')
  } finally {
    downloading.value = null
  }
}

async function deleteModel(code: string): Promise<void> {
  const { push, localizeError } = useToast()
  try {
    await invoke('delete_ocr_model', { language: code })
    // 从已选集合移除，避免继续引用已删除模型
    const idx = selected.value.indexOf(code)
    if (idx >= 0) selected.value.splice(idx, 1)
    push(t('ocr.deleted'), 'success')
    await refresh()
  } catch (err) {
    push(localizeError(err, t('ocr.verifyFailed')), 'error')
  }
}

export function useOcrLanguages() {
  return {
    languages,
    loading,
    downloading,
    progress,
    failed,
    selected,
    builtinLanguages,
    downloadableLanguages,
    plannedLanguages,
    isSelected,
    toggle,
    refresh,
    downloadModel,
    deleteModel,
  }
}
