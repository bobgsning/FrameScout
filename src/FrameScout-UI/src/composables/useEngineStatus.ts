/**
 * useEngineStatus — 启动握手：监听 Rust 端 engine-status 事件，驱动加载屏。
 * 引擎就绪后顺带拉取智能文件夹列表。
 */
import { ref, onMounted, onUnmounted } from 'vue'
import { listen } from '@tauri-apps/api/event'
import type { EngineStatusPayload, EngineStatusValue } from '@/types/license'
import { useSmartFolders } from './useSmartFolders'

// ---------- 模块级共享状态 ----------
const engineReady = ref(false)
const engineStatus = ref<EngineStatusValue>('connecting')
const engineRetry = ref(0)
const engineMaxRetries = ref(50)
const engineMessage = ref('Connecting to AI engine...')

let unlistenEngineStatus: (() => void) | null = null

/** 保证 engine-status 只订阅一次 */
let lifecycleBound = false

export function useEngineStatus() {
  const { loadSmartFolders } = useSmartFolders()

  if (!lifecycleBound) {
    lifecycleBound = true

    onMounted(async () => {
      // 清理历史遗留的本地缓存（智能文件夹已改为后端持久化）
      localStorage.removeItem('framescout_smart_folders')
      localStorage.removeItem('framescout_folder_path')

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
            engineMaxRetries.value = payload.max_retries ?? 50
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
