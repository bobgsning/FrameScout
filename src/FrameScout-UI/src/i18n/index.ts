import { createI18n } from 'vue-i18n'
import en from './locales/en.json'
import zhCN from './locales/zh-CN.json'
import zhTW from './locales/zh-TW.json'
import ja from './locales/ja.json'
import ru from './locales/ru.json'
import ko from './locales/ko.json'
import de from './locales/de.json'
import fr from './locales/fr.json'
import es from './locales/es.json'

export const SUPPORTED_LOCALES = ['en', 'zh-CN', 'zh-TW', 'ja', 'ru', 'ko', 'de', 'fr', 'es'] as const
export type AppLocale = (typeof SUPPORTED_LOCALES)[number]

export const DEFAULT_LOCALE: AppLocale = 'en'

const messages = {
  en,
  'zh-CN': zhCN,
  'zh-TW': zhTW,
  ja,
  ru,
  ko,
  de,
  fr,
  es,
}

export const i18n = createI18n({
  legacy: false,
  globalInjection: true,
  locale: DEFAULT_LOCALE,
  fallbackLocale: DEFAULT_LOCALE,
  messages,
})

/** 切换 UI 语言；未知值回退到英文。 */
export function setLocale(locale: string): AppLocale {
  const next: AppLocale = (SUPPORTED_LOCALES as readonly string[]).includes(locale)
    ? (locale as AppLocale)
    : DEFAULT_LOCALE
  i18n.global.locale.value = next
  if (typeof document !== 'undefined') {
    document.documentElement.lang = next
  }
  return next
}

/** 后端码的形态：`ERR_XXX` / `STATUS_XXX` / `TAG_XXX` / `MSG_XXX`（全大写 + 下划线）。 */
const CODE_RE = /^(ERR|STATUS|TAG|MSG)_[A-Z0-9_]+$/

/**
 * 把后端回传的码（或任意字符串）解析成本地化文案。
 *
 * 传输格式（与 Rust 侧 `src-tauri/src/error.rs` 对齐）：`CODE` 或 `CODE|参数0|参数1...`。
 * 不是码的内容（真实文件路径、普通句子、空串）会原样返回——这让它可以安全地用在
 * `scan-progress.file_path` 这类「有时是路径、有时是状态文案」的字段上。
 * 查不到文案时同样返回原串，绝不把裸 key 显示给用户。
 */
export function localizeCode(raw: unknown): string {
  if (raw == null) return ''
  const s = String(raw)
  if (!s) return ''

  const idx = s.indexOf('|')
  const code = idx === -1 ? s : s.slice(0, idx)
  if (!CODE_RE.test(code)) return s

  const params = idx === -1 ? [] : s.slice(idx + 1).split('|')
  const key = `backend.${code}`
  // 直接走 i18n.global.t，避免与下面 t() 对 {err} 的自动解析相互递归
  const out = i18n.global.t(key as never, {
    '0': params[0] ?? '',
    '1': params[1] ?? '',
    '2': params[2] ?? '',
    detail: params.join('|'),
  } as never)
  return out === key ? s : out
}

/** 供非组件环境（.ts 工具函数 / 组合式函数）使用的翻译函数。 */
export function t(key: string, params?: Record<string, unknown>): string {
  const resolved = params ? { ...params } : undefined
  // 后端错误码普遍以 {err} 直接插进模板（如 "备份失败：{err}"）。
  // 在这里统一解析，调用点就不必逐个包一层，避免漏改导致用户看到裸码。
  if (resolved && typeof resolved.err !== 'undefined') {
    resolved.err = localizeCode(resolved.err)
  }
  // vue-i18n 的 key 为强类型 Path，此处按运行时字符串处理
  return i18n.global.t(key as never, resolved as never)
}
