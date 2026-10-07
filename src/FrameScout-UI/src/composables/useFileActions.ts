/**
 * useFileActions — 文件操作工具（P1-1 / 第三轮「依赖装了却完全没用」）。
 *
 * `@tauri-apps/plugin-opener` 早在 package.json 声明，但全仓库 0 import。
 * 本组合式把它接上：打开文件 / 在资源管理器中显示 / 复制路径。
 * 这是「全项目性价比最高的一行依赖」——一行 import 兑现三个日常动作。
 */
import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener'
import { useToast } from './useToast'
import { t } from '@/i18n'

/** 用浏览器剪贴板 API 复制文本（Tauri webview 支持 navigator.clipboard）。 */
async function copyTextToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text)
    return true
  } catch {
    // 兜底：execCommand（旧 webview 或 clipboard 权限被拒时）
    try {
      const ta = document.createElement('textarea')
      ta.value = text
      ta.style.position = 'fixed'
      ta.style.opacity = '0'
      document.body.appendChild(ta)
      ta.select()
      const ok = document.execCommand('copy')
      document.body.removeChild(ta)
      return ok
    } catch {
      return false
    }
  }
}

export function useFileActions() {
  /** 用系统默认程序打开文件（图片用看图器、视频用播放器）。 */
  async function openFile(path: string): Promise<boolean> {
    try {
      await openPath(path)
      return true
    } catch (err) {
      // P1-10：打开文件失败此前静默；现给可见反馈
      const { push } = useToast()
      push(t('file.openFailed', { err }), 'error')
      return false
    }
  }

  /** 在资源管理器中定位到该文件（高亮选中）。 */
  async function revealInExplorer(path: string): Promise<boolean> {
    try {
      await revealItemInDir(path)
      return true
    } catch (err) {
      // P1-10：显示位置失败此前静默；现给可见反馈
      const { push } = useToast()
      push(t('file.revealFailed', { err }), 'error')
      return false
    }
  }

  /** 复制文件路径到剪贴板。 */
  async function copyPath(path: string): Promise<boolean> {
    return copyTextToClipboard(path)
  }

  return {
    openFile,
    revealInExplorer,
    copyPath,
  }
}
