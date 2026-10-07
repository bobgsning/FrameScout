/**
 * useEngineStatus — 启动握手：监听 Rust 端 engine-status 事件，驱动加载屏。
 * 引擎就绪后顺带拉取智能文件夹列表。
 */
import { ref, onMounted, onUnmounted } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { t } from '@/i18n'
import type { EngineStatusPayload, EngineStatusValue } from '@/types/license'
import { useSmartFolders } from './useSmartFolders'

// ---------- 模块级共享状态 ----------
const engineReady = ref(false)
const engineStatus = ref<EngineStatusValue>('connecting')
const engineRetry = ref(0)
const engineMaxRetries = ref(30)
const engineMessage = ref(t('splash.connecting'))

let unlistenEngineStatus: (() => void) | null = null

/** 保证 engine-status 只订阅一次 */
let lifecycleBound = false

export function useEngineStatus() {
  const { loadSmartFolders } = useSmartFolders()

  if (!lifecycleBound) {
    lifecycleBound = true

    onMounted(async () => {
      // 清理历史遗留的智能文件夹缓存（已改为后端持久化）。
      // **不再清除 folder_path**（P1-5 修复）：旧实现每次启动都删 folder_path，
      // 导致用户每天重开 App 都要重新粘贴素材库路径。现在 folder_path 由
      // usePreferences 持久化管理，重开 App 自动回填。
      localStorage.removeItem('framescout_smart_folders')

      unlistenEngineStatus = await listen<EngineStatusPayload>('engine-status', (event) => {
        const payload = event.payload
        switch (payload.status) {
          case 'ready':
            engineReady.value = true
            engineStatus.value = 'ready'
            engineMessage.value = ''
            loadSmartFolders()
            break
          case 'connecting':
            engineStatus.value = 'connecting'
            engineRetry.value = payload.retry ?? 0
            engineMaxRetries.value = payload.max_retries ?? 30
            engineMessage.value = payload.message
            break
          case 'error':
            engineStatus.value = 'error'
            engineMessage.value = payload.message
            break
        }
      })
    })

    onUnmounted(() => {
      if (unlistenEngineStatus) {
        unlistenEngineStatus()
        unlistenEngineStatus = null
      }
      // HMR：允许重新挂载时再次订阅 engine-status
      lifecycleBound = false
    })
  }

  return {
    engineReady,
    engineStatus,
    engineMessage,
    engineRetry,
    engineMaxRetries
  }
}
