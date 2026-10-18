/**
 * useClip — 视频「切片段」（P2-9 / 死命令 cut_video_clip 接线）。
 *
 * 视频结果命中某一帧后，用户想把「包含命中时刻的一小段」单独截出来。这里维护
 * 弹层所需的最小状态机：pendingItem（要切的条目）+ duration（默认 5s，可调），
 * confirmClip 调用后端 `cut_video_clip`，成功/失败走全局 Toast。
 *
 * 状态为模块级单例：卡片、全屏详情、弹层三处共享同一份 pendingItem / duration。
 */
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from './useToast'
import { usePreferences } from './usePreferences'
import { t } from '@/i18n'
import { clipOutputPath } from '@/utils/media'
import type { SearchResult } from '@/types/search'

const pendingItem = ref<SearchResult | null>(null)
const duration = ref(5)

export function useClip() {
  const { push } = useToast()
  const { exportPath } = usePreferences()

  /** 弹层打开：记录要切的条目，时长回默认 5s。 */
  function requestClip(item: SearchResult) {
    pendingItem.value = item
    duration.value = 5
  }

  function cancelClip() {
    pendingItem.value = null
  }

  async function confirmClip() {
    const item = pendingItem.value
    if (!item) return
    pendingItem.value = null

    const dur = Math.max(0.5, Number(duration.value) || 5)
    // 命中帧前后各半：起点提前 dur/2，下限 0。
    const startSec = Math.max(0, item.timestamp - dur / 2)
    // 导出目录优先，未设置则回退源视频同目录。
    const outputPath = clipOutputPath(item.path, item.timestamp, exportPath.value)

    try {
      const saved = await invoke<string>('cut_video_clip', {
        inputPath: item.path,
        outputPath,
        startSec,
        durationSec: dur,
      })
      push(t('clip.saved', { path: saved }), 'success')
    } catch (err) {
      push(t('clip.failed', { err }), 'error')
    }
  }

  return { pendingItem, duration, requestClip, cancelClip, confirmClip }
}
