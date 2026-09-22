/**
 * Search keyword highlighting.
 *
 * 注意：返回的字符串通过 v-html 注入 DOM。
 * 必须先转义 HTML，再做关键词替换，否则原始文本里的标签会被当作真实 DOM 渲染（XSS 面）。
 */

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

/** 转义正则元字符，避免用户输入 "("、"*"、"[" 时抛 SyntaxError */
function escapeRegExp(input: string): string {
  return input.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

/**
 * 高亮关键词
 * @param text          原文（路径 / OCR 文本）
 * @param query         当前搜索词
 * @param isImageSearch 以图搜图模式下不做高亮（没有文本关键词）
 */
export function highlight(text: string, query: string, isImageSearch = false): string {
  if (!text || !query || isImageSearch) return text || ''

  const safeText = escapeHtml(text)
  const regex = new RegExp(`(${escapeRegExp(query)})`, 'gi')
  return safeText.replace(regex, '<span class="highlight-text">$1</span>')
}
