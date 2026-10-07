import { createI18n } from 'vue-i18n'
import en from './locales/en.json'
import zhCN from './locales/zh-CN.json'

export const SUPPORTED_LOCALES = ['en', 'zh-CN'] as const
export type AppLocale = (typeof SUPPORTED_LOCALES)[number]

export const DEFAULT_LOCALE: AppLocale = 'en'

export const i18n = createI18n({
  legacy: false,
  globalInjection: true,
  locale: DEFAULT_LOCALE,
  fallbackLocale: DEFAULT_LOCALE,
  messages: {
    en,
    'zh-CN': zhCN,
  },
})

/** 切换 UI 语言；未知值回退到英文。 */
export function setLocale(locale: string): AppLocale {
  const next: AppLocale = locale === 'zh-CN' ? 'zh-CN' : 'en'
  i18n.global.locale.value = next
  if (typeof document !== 'undefined') {
    document.documentElement.lang = next
  }
  return next
}

/** 供非组件环境（.ts 工具函数 / 组合式函数）使用的翻译函数。 */
export function t(key: string, params?: Record<string, unknown>): string {
  // vue-i18n 的 key 为强类型 Path，此处按运行时字符串处理
  return i18n.global.t(key as never, params as never)
}
