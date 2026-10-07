/**
 * Thin wrappers around Tauri IPC commands that carry no UI state.
 * （有状态的业务流请走 composables/，这里只放「调一次就完事」的原子操作）
 *
 * P1-8 / P1-13 修复（第三轮）：
 *   - 所有 alert() 改走 useToast；
 *   - saveNote 加成功/失败反馈（不再静默丢数据）。
 */
import { invoke } from '@tauri-apps/api/core'
import type { SearchResult } from '@/types/search'
import { useToast } from '@/composables/useToast'
import { t } from '@/i18n'

/** 保存用户 Markdown 笔记（失焦即存）。P1-13：加成功/失败反馈。 */
export async function saveNote(path: string, note: string, timestamp?: number): Promise<boolean> {
  const { push, localizeError } = useToast()
  try {
    await invoke('update_note', { path, note, timestamp })
    return true
  } catch (err) {
    const msg = localizeError(err, t('api.saveNoteFailed'))
    push(msg, 'error')
    return false
  }
}

/**
 * 对单个结果即时跑 OCR
 * @param item          目标结果（就地修改 ocrRunning 标记）
 * @param fallbackLangs 卡片未单独指定语言时，回退到扫描栏的全局语言设置
 */
export async function runOcrForItem(item: SearchResult, fallbackLangs: string): Promise<void> {
  const { push, localizeError } = useToast()
  const raw = item.ocrLangInput || fallbackLangs
  const langs = raw
    .split(',')
    .map((l) => l.trim())
    .filter(Boolean)

  if (langs.length === 0) {
    push(t('api.ocrLangRequired'), 'warning')
    return
  }

  item.ocrRunning = true
  try {
    const updated: number = await invoke('run_ocr_for_selected_files', {
      filePaths: [item.path],
      languages: langs
    })

    if (updated > 0) {
      push(t('api.ocrUpdated', { path: item.path }), 'success')
    } else {
      push(t('api.ocrNotUpdated'), 'warning')
    }
  } catch (err) {
    push(localizeError(err, t('api.ocrFailed')), 'error')
  } finally {
    item.ocrRunning = false
  }
}

/** 统一的语言串解析："en, ch_sim" -> ["en", "ch_sim"] */
export function parseLanguages(input: string): string[] {
  return input
    .split(',')
    .map((l) => l.trim())
    .filter(Boolean)
}
