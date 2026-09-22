/**
 * useScanner — 目录/文件索引、scan-progress 事件总线、新入库文件提醒、幽灵记录清理。
 */
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import type { ScanProgress, SearchResult } from '@/types/search'
import type { CleanGhostsResult, ScanProgressPayload } from '@/types/license'
import { parseLanguages } from '@/utils/api'
import { useSearch } from './useSearch'
import { useClustering } from './useClustering'

// ---------- 模块级共享状态 ----------
const folderPath = ref('')
const scanMode = ref('all')
const enableOcr = ref(false)
const ocrLanguages = ref('en') // 默认英文，可填 "en, ch_sim, ja"

const isScanning = ref(false)
const scanMsg = ref('')
const currentStatus = ref('')
const currentFile = ref('')
const scanProgress = ref<ScanProgress>({ current: 0, total: 0 })

const incomingFiles = ref<SearchResult[]>([])
const showIncomingBanner = ref(false)
const incomingCount = computed(() => incomingFiles.value.length)

let unlistenScanProgress: (() => void) | null = null

/** 保证 scan-progress 只订阅一次 */
let lifecycleBound = false

export function useScanner() {
  const {
    searchQuery,
    isImageSearch,
    isShowAllMode,
    currentPage,
    jumpPage,
    results,
    totalResults,
    loadBrowsePage,
    performSearch,
    showAllFiles
  } = useSearch()
  const { isClusteringView } = useClustering()

  async function selectFolder() {
    const selected = await open({ directory: true })
    if (selected) {
      folderPath.value = selected as string
    }
  }

  async function startScan() {
    // 清掉上一轮扫描遗留的待入库文件
    incomingFiles.value = []
    showIncomingBanner.value = false

    if (!folderPath.value) return

    isScanning.value = true
    scanMsg.value = 'Extracting high-dimensional features...'
    try {
      const count: number = await invoke('scan_folder', {
        folderPath: folderPath.value,
        scanMode: scanMode.value,
        enableOcr: enableOcr.value,
        ocrLanguages: parseLanguages(ocrLanguages.value)
      })

      scanMsg.value =
        count === 0
          ? `✅ Scan completed! No new files found. Memory Matrix is up to date.`
          : `✅ Memory loaded! Successfully extracted ${count} new spatio-temporal slices.`
    } catch (err) {
      scanMsg.value = `❌ Failed: ${err}`
    } finally {
      isScanning.value = false
      // 仅在浏览模式下刷新列表，避免打断用户正在看的搜索结果
      if (!searchQuery.value && !isImageSearch.value && !isClusteringView.value) {
        await loadBrowsePage(currentPage.value)
      }
    }
  }

  async function selectAndIndexFiles() {
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: 'Media',
          extensions: ['jpg', 'jpeg', 'png', 'webp', 'mp4', 'mov', 'avi', 'mkv', 'webm', 'flv']
        }
      ]
    })
    if (!selected || selected.length === 0) return

    isScanning.value = true
    scanMsg.value = `Indexing ${selected.length} files...`
    try {
      const count: number = await invoke('index_files', {
        filePaths: selected,
        enableOcr: enableOcr.value,
        ocrLanguages: parseLanguages(ocrLanguages.value)
      })
      scanMsg.value = `✅ Indexed ${count} new files.`

      if (!searchQuery.value && !isImageSearch.value && !isClusteringView.value) {
        await loadBrowsePage(currentPage.value)
      }
    } catch (err) {
      scanMsg.value = `❌ Indexing failed: ${err}`
    } finally {
      isScanning.value = false
    }
  }

  async function cleanGhosts() {
    if (
      !confirm('Are you sure you want to purge records of physically deleted files from the database?')
    )
      return

    try {
      const result: CleanGhostsResult = await invoke('clean_ghosts')
      const count = result?.removed_count ?? 0
      alert(`🧹 Successfully purged ${count} ghost records!`)

      const removedSet = new Set(result?.removed_paths ?? [])
      results.value = results.value.filter((r) => !removedSet.has(r.path))
      totalResults.value = Math.max(0, totalResults.value - count)

      if (!searchQuery.value && !isImageSearch.value && !isClusteringView.value) {
        await loadBrowsePage(currentPage.value)
      } else {
        await performSearch()
      }
    } catch (err) {
      alert('Purge failed: ' + err)
    }
  }

  async function acceptIncomingFiles() {
    // Show All 模式：新文件已入库，直接重载全量列表
    if (isShowAllMode.value) {
      await showAllFiles()
      incomingFiles.value = []
      showIncomingBanner.value = false
      return
    }

    // 退出搜索/聚类态，切回浏览模式
    searchQuery.value = ''
    isImageSearch.value = false
    isClusteringView.value = false

    currentPage.value = 1
    jumpPage.value = 1

    // list_all_files 按时间戳倒序，最新文件排在最前
    await loadBrowsePage(currentPage.value)

    incomingFiles.value = []
    showIncomingBanner.value = false
  }

  function dismissIncomingBanner() {
    // 只隐藏，不清空 incomingFiles：下次扫描仍会提示
    showIncomingBanner.value = false
  }

  // useScanner 也会被 TopActionBar / OcrPanel 调用，
  // 守卫确保 scan-progress 只订阅一次（否则同一事件会被重复处理）
  if (!lifecycleBound) {
    lifecycleBound = true

    onMounted(async () => {
      unlistenScanProgress = await listen<ScanProgressPayload>('scan-progress', (e) => {
        const payload = e.payload
        currentStatus.value = payload.status
        currentFile.value = payload.file_path
        scanProgress.value = { current: payload.current, total: payload.total }

        if (payload.new_files && payload.new_files.length > 0) {
          const newItems: SearchResult[] = payload.new_files.map((newPath) => ({
            path: newPath,
            timestamp: 0.0,
            score: 2.0, // 哨兵值：新入库文件直接显示 100%
            matched_tags: ['✨ Fresh Index'],
            ocr_text: '',
            user_note: '',
            isMissing: false,
            expandOcr: false
          }))

          // 去重后再入列
          newItems.forEach((item) => {
            if (!incomingFiles.value.some((f) => f.path === item.path)) {
              incomingFiles.value.push(item)
            }
          })
          showIncomingBanner.value = true
        }
      })
    })

    onUnmounted(() => {
      if (unlistenScanProgress) {
        unlistenScanProgress()
        unlistenScanProgress = null
      }
      // HMR：允许重新挂载时再次订阅 scan-progress
      lifecycleBound = false
    })
  }

  return {
    // state
    folderPath,
    scanMode,
    enableOcr,
    ocrLanguages,
    isScanning,
    scanMsg,
    currentStatus,
    currentFile,
    scanProgress,
    incomingFiles,
    showIncomingBanner,
    incomingCount,
    // actions
    selectFolder,
    startScan,
    selectAndIndexFiles,
    cleanGhosts,
    acceptIncomingFiles,
    dismissIncomingBanner
  }
}
