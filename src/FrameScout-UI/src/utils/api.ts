/**
 * Thin wrappers around Tauri IPC commands that carry no UI state.
 * （有状态的业务流请走 composables/，这里只放「调一次就完事」的原子操作）
 */
import { invoke } from '@tauri-apps/api/core'
import type { SearchResult } from '@/types/search'

/** 保存用户 Markdown 笔记（失焦即存） */
export async function saveNote(path: string, note: string): Promise<void> {
  try {
    await invoke('update_note', { path, note })
  } catch (err) {
    console.error('Note save failed', err)
  }
}

/**
 * 对单个结果即时跑 OCR
 * @param item          目标结果（就地修改 ocrRunning 标记）
 * @param fallbackLangs 卡片未单独指定语言时，回退到扫描栏的全局语言设置
 */
export async function runOcrForItem(item: SearchResult, fallbackLangs: string): Promise<void> {
  const raw = item.ocrLangInput || fallbackLangs
  const langs = raw
    .split(',')
    .map((l) => l.trim())
    .filter(Boolean)

  if (langs.length === 0) {
    alert('Please specify at least one language.')
    return
  }

  item.ocrRunning = true
  try {
    const updated: number = await invoke('run_ocr_for_selected_files', {
      filePaths: [item.path],
      languages: langs
    })

    if (updated > 0) {
      alert(`✅ OCR updated for ${item.path}. Please refresh results to see changes.`)
    } else {
      alert(
        '⚠️ No OCR update performed. The file may already have OCR text or the engine did not return results.'
      )
    }
  } catch (err) {
    alert(`OCR failed: ${err}`)
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
