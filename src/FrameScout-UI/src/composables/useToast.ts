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
import { localizeBackend } from '@/utils/backendText'

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

/**
 * 把后端错误码本地化为友好文案。
 *
 * 后端现在只回稳定错误码（见 `src-tauri/src/error.rs`），由 `localizeBackend`
 * 按码查 i18n 的 `backend.*`；查不到（例如第三方库的原生英文错误）才退回到
 * 「本地化前缀 + 截断原文」，保证用户至少能拿到可诊断的信息。
 */
function localizeError(err: unknown, fallback: string): string {
  if (err == null) return fallback
  const s = String(err)
  const localized = localizeBackend(s)
  if (localized !== s) return localized
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
