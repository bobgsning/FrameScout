/**
 * useKeyboard — 全局键盘快捷键（P1-8 / 第三轮「几乎无键盘快捷键」）。
 *
 * 桌面应用的肌肉记忆：
 *   - Esc：关闭最上层弹窗（按优先级关闭一个，而非全部）
 *   - `/`：聚焦搜索框
 *   - ←/→：翻页（上一页/下一页）
 *   - Space：Lightbox 下一张（仅 Lightbox 打开时）
 *
 * 用法：App.vue 调用 useKeyboard(dialogStack) 注册监听。
 * dialogStack 是一个返回当前打开的对话框列表的函数，按 z-index 降序。
 */
import { onMounted, onUnmounted } from 'vue'

export interface CloseableDialog {
  name: string
  close: () => void
}

export function useKeyboard(
  getDialogStack: () => CloseableDialog[],
  options: {
    focusSearch?: () => void
    prevPage?: () => void
    nextPage?: () => void
    lightboxNext?: () => void
    lightboxPrev?: () => void
    isLightboxOpen?: () => boolean
  } = {}
) {
  function onKeydown(e: KeyboardEvent) {
    // 在输入框/textarea 中不拦截（Esc 除外，让用户能退出输入态）
    const target = e.target as HTMLElement
    const isInput = target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)

    // Esc：关闭最上层弹窗
    if (e.key === 'Escape') {
      const stack = getDialogStack()
      if (stack.length > 0) {
        // 按 z-index 降序，关最上层的那个
        const top = stack[stack.length - 1]
        top.close()
        e.preventDefault()
      }
      return
    }

    // 输入态下不拦截其余快捷键
    if (isInput) return

    // `/`：聚焦搜索框
    if (e.key === '/' && options.focusSearch) {
      options.focusSearch()
      e.preventDefault()
      return
    }

    // Lightbox 打开时的 Space / 方向键
    if (options.isLightboxOpen && options.isLightboxOpen()) {
      if (e.key === ' ' || e.code === 'Space') {
        if (options.lightboxNext) options.lightboxNext()
        e.preventDefault()
        return
      }
      if (e.key === 'ArrowLeft' && options.lightboxPrev) {
        options.lightboxPrev()
        e.preventDefault()
        return
      }
      if (e.key === 'ArrowRight' && options.lightboxNext) {
        options.lightboxNext()
        e.preventDefault()
        return
      }
      return // Lightbox 打开时不响应翻页
    }

    // ←/→ 翻页
    if (e.key === 'ArrowLeft' && options.prevPage) {
      options.prevPage()
      e.preventDefault()
      return
    }
    if (e.key === 'ArrowRight' && options.nextPage) {
      options.nextPage()
      e.preventDefault()
      return
    }
  }

  onMounted(() => {
    window.addEventListener('keydown', onKeydown)
  })

  onUnmounted(() => {
    window.removeEventListener('keydown', onKeydown)
  })
}
