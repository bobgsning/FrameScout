/**
 * useToast — 全局 Toast 通知体系（P1-8 / 第三轮「无全局 Toast 体系」）。
 *
 * 旧实现：错误消息散落在 5 套 CSS 类、4 处原生 alert()、多处 console.error。
 * 新实现：统一 Toast 队列，自动消失，右上角堆叠，三种语义色。
 *
 * 用法：
 *   const { push } = useToast()
 *   push('已索引 137 个文件', 'success')
 *   push('保存失败：网络超时', 'error')
 *   push('正在处理大视频…', 'info', 0)  // 0 = 不自动消失
 */
import { ref } from 'vue'
import { t } from '@/i18n'

export type ToastKind = 'success' | 'error' | 'info' | 'warning'

export interface ToastAction {
  /** 按钮文案 */
  label: string
  /** 点击回调 */
  onClick: () => void
}

export interface ToastItem {
  id: number
  message: string
  kind: ToastKind
  /** 自动消失毫秒数；0 = 不自动消失 */
  duration: number
  /** P2-1：可选的操作按钮（如「撤销」） */
  action?: ToastAction
  /** P2-6：剩余存活毫秒（用于进度条展示） */
  remaining?: number
}

const toasts = ref<ToastItem[]>([])
let nextId = 0

/** P2-6：Toast 堆叠上限——超出后移除最早的一条，避免刷满整屏 */
const MAX_TOASTS = 5

const DEFAULT_DURATION: Record<ToastKind, number> = {
  success: 3000,
  error: 6000,
  info: 3500,
  warning: 4500,
}

function push(message: string, kind: ToastKind = 'info', duration?: number, action?: ToastAction): number {
  const id = ++nextId
  const dur = duration ?? DEFAULT_DURATION[kind]
  const item: ToastItem = { id, message, kind, duration: dur }
  if (action) item.action = action
  toasts.value.push(item)
  // 堆叠上限：超出则移除最早的一条
  while (toasts.value.length > MAX_TOASTS) {
    toasts.value.shift()
  }
  if (dur > 0) {
    const start = Date.now()
    item.remaining = dur
    const tick = setInterval(() => {
      const elapsed = Date.now() - start
      item.remaining = Math.max(0, dur - elapsed)
      if (item.remaining <= 0) clearInterval(tick)
    }, 100)
    setTimeout(() => dismiss(id), dur)
  }
  return id
}

function dismiss(id: number): void {
  const idx = toasts.value.findIndex((t) => t.id === id)
  if (idx >= 0) toasts.value.splice(idx, 1)
}

function clear(): void {
  toasts.value = []
}

// 便捷方法
function success(msg: string, dur?: number) { return push(msg, 'success', dur) }
function error(msg: string, dur?: number) { return push(msg, 'error', dur) }
function info(msg: string, dur?: number) { return push(msg, 'info', dur) }
function warning(msg: string, dur?: number) { return push(msg, 'warning', dur) }

/** 把 Rust 错误字符串本地化为友好中文文案（P1-8 / 第三轮「错误文案直接甩 Rust 错误」）。 */
function localizeError(err: unknown, fallback: string): string {
  if (err == null) return fallback
  const s = String(err)
  // 常见 Rust/SQLite 错误模式本地化
  if (s.includes('database is locked')) return t('toast.databaseLocked')
  if (s.includes('Memory Matrix is empty')) return t('toast.emptyMatrix')
  if (s.includes('No paths selected')) return t('toast.noPaths')
  if (s.includes('Smart folder not found')) return t('toast.smartFolderNotFound')
  if (s.includes('BGE-M3 text engine not available')) return t('toast.textEngineUnavailable')
  if (s.includes('Failed to extract image vectors')) return t('toast.imageVectorFailed')
  if (s.includes('Aborted after') && s.includes('consecutive failures')) return t('toast.abortedFailures')
  if (s.includes('Trial Limit Reached')) return t('toast.trialLimit')
  // 兜底：截断过长的错误串
  return `${fallback}${t('toast.errorSeparator')}${s.length > 120 ? s.slice(0, 120) + '…' : s}`
}

export function useToast() {
  return {
    toasts,
    push,
    dismiss,
    clear,
    success,
    error,
    info,
    warning,
    localizeError,
  }
}
