/**
 * exporters — 结果导出工具（P1-10 / P2-9 / 第三轮「无导出结果 / 分享」）。
 *
 * 三种格式：
 *   - CSV：给客户/Excel 用
 *   - Markdown：给同事/文档用
 *   - JSON：完整结构化数据
 *
 * 用法：
 *   import { exportCSV, exportMarkdown, exportJSON } from '@/utils/exporters'
 *   exportCSV(results, 'search_results.csv')
 */
import type { SearchResult } from '@/types/search'
import { isVideo } from '@/utils/media'
import { getSearchNumericScore, getNumericScore } from '@/utils/score'
import { t } from '@/i18n'
import { useToast } from '@/composables/useToast'
import { usePreferences } from '@/composables/usePreferences'
import { invoke } from '@tauri-apps/api/core'

/** 导出的分数列：统一为人类可读分（0~99.9）。 */
function humanScore(r: SearchResult): string {
  const num = r.scoreKind === 'bge' ? getNumericScore(r.score, false, 'bge') : getSearchNumericScore(r)
  return num.toFixed(1)
}

async function downloadBlob(content: string, filename: string, mime: string): Promise<void> {
  const { push } = useToast()
  // 用户在设置里配了导出目录 → 直接写文件，Toast 提示完整路径
  const { exportPath } = usePreferences()
  const dir = exportPath.value.trim()
  if (dir) {
    try {
      const fullPath = await invoke<string>('export_file', { dir, filename, content })
      push(t('export.savedTo', { path: fullPath }), 'success')
    } catch (err) {
      push(t('export.exportFailed', { err }), 'error')
    }
    return
  }

  // 未设置导出目录 → 回退浏览器下载（P2-2：加 try/catch，失败可见 Toast）
  try {
    const blob = new Blob([content], { type: `${mime};charset=utf-8` })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = filename
    a.style.display = 'none'
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    setTimeout(() => URL.revokeObjectURL(url), 1000)
  } catch (err) {
    push(t('export.exportFailed', { err }), 'error')
  }
}

/** P2-2：截断并加「已截断」标记——用户不再拿到一份静默不完整的导出 */
function truncateWithMark(text: string, max: number): string {
  if (text.length <= max) return text
  return text.slice(0, max) + t('export.truncatedMark')
}

function escapeCSV(val: string): string {
  if (val.includes(',') || val.includes('"') || val.includes('\n')) {
    return `"${val.replace(/"/g, '""')}"`
  }
  return val
}

function fileName(path: string): string {
  const parts = path.replace(/\\/g, '/').split('/')
  return parts[parts.length - 1] || path
}

function dirName(path: string): string {
  const parts = path.replace(/\\/g, '/').split('/')
  parts.pop()
  return parts.join('/') || path
}

/** 导出为 CSV */
export async function exportCSV(results: SearchResult[], filename = 'framescout_results.csv'): Promise<void> {
  const header = [t('export.no'), t('export.fileName'), t('export.fullPath'), t('export.directory'), t('export.type'), t('export.score'), t('export.ocrText'), t('export.note'), t('export.matchCount')]
  const rows = results.map((r, i) => [
    String(i + 1),
    fileName(r.path),
    r.path,
    dirName(r.path),
    isVideo(r.path) ? t('export.video') : t('export.image'),
    humanScore(r),
    truncateWithMark((r.ocr_text || '').replace(/\n/g, ' '), 500),
    truncateWithMark((r.user_note || '').replace(/\n/g, ' '), 500),
    String(r.match_count || 0),
  ].map(escapeCSV).join(','))

  const csv = '\uFEFF' + [header.join(','), ...rows].join('\n')
  await downloadBlob(csv, filename, 'text/csv')
}

/** 导出为 Markdown */
export async function exportMarkdown(results: SearchResult[], filename = 'framescout_results.md'): Promise<void> {
  const lines: string[] = [
    t('export.mdTitle'),
    '',
    t('export.exportTime', { time: new Date().toLocaleString() }),
    t('export.resultCount', { count: results.length }),
    '',
  ]

  for (let i = 0; i < results.length; i++) {
    const r = results[i]
    lines.push(`## ${i + 1}. ${fileName(r.path)}`)
    lines.push('')
    lines.push(t('export.path', { path: r.path }))
    lines.push(t('export.typeLine', { type: isVideo(r.path) ? t('export.video') : t('export.image') }))
    if (r.timestamp > 0) lines.push(t('export.frameTime', { time: r.timestamp.toFixed(1) }))
    lines.push(t('export.scoreLine', { score: humanScore(r) }))
    if (r.match_count) lines.push(t('export.matchCountLine', { count: r.match_count }))
    if (r.matched_tags?.length) lines.push(t('export.matchedChannels', { channels: r.matched_tags.join(' / ') }))
    if (r.ocr_text) {
      lines.push('')
      lines.push(t('export.ocrSection'))
      lines.push('```')
      lines.push(truncateWithMark(r.ocr_text, 1000))
      lines.push('```')
    }
    if (r.user_note) {
      lines.push('')
      lines.push(t('export.noteSection'))
      lines.push(r.user_note)
    }
    lines.push('')
    lines.push('---')
    lines.push('')
  }

  await downloadBlob(lines.join('\n'), filename, 'text/markdown')
}

/** 导出为 JSON */
export async function exportJSON(results: SearchResult[], filename = 'framescout_results.json'): Promise<void> {
  const payload = {
    exported_at: new Date().toISOString(),
    count: results.length,
    results: results.map((r) => ({
      path: r.path,
      file_name: fileName(r.path),
      dir: dirName(r.path),
      type: isVideo(r.path) ? 'video' : 'image',
      timestamp: r.timestamp,
      score: r.score,
      human_score: humanScore(r),
      matched_tags: r.matched_tags,
      ocr_text: r.ocr_text,
      user_note: r.user_note,
      match_count: r.match_count || 0,
      ocr_lines: r.ocr_lines || [],
      index_time: r.index_time,
    })),
  }
  await downloadBlob(JSON.stringify(payload, null, 2), filename, 'application/json')
}
