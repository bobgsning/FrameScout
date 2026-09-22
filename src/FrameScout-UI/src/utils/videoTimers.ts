/**
 * Video load timeout registry.
 *
 * 原实现把 videoTimers 放在组件作用域内，组件卸载时统一清理。
 * 这里提升为模块级单例：useSearch（发起搜索时武装计时器）与 VideoCard（加载成功/失败时解除）
 * 都能访问同一份注册表，避免通过 props 层层传递。
 */

export const VIDEO_LOAD_TIMEOUT_MS = 25_000

const videoTimers = new Map<string, number>()

/** 武装超时：超时后仍未 loadeddata 就标记为文件不可达 */
export function armVideoTimeout(path: string, onTimeout: () => void, ms = VIDEO_LOAD_TIMEOUT_MS): void {
  clearVideoTimer(path)
  const timerId = window.setTimeout(() => {
    videoTimers.delete(path)
    onTimeout()
  }, ms)
  videoTimers.set(path, timerId)
}

/** 解除计时器（视频加载成功或已明确报错） */
export function clearVideoTimer(path: string): void {
  const timerId = videoTimers.get(path)
  if (timerId !== undefined) {
    clearTimeout(timerId)
    videoTimers.delete(path)
  }
}

/** 组件卸载 / 新一轮搜索开始时全量清理，防止旧的计时器误伤新结果 */
export function clearAllVideoTimers(): void {
  for (const [, timerId] of videoTimers) {
    clearTimeout(timerId)
  }
  videoTimers.clear()
}
