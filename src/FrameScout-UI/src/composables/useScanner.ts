/**
 * useScanner — 目录/文件索引、scan-progress 事件总线、新入库文件提醒、幽灵记录清理。
 */
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import type {
  BatchReport,
  DiffReport,
  GhostActionResult,
  GhostItem,
  ReindexResult,
  ScanProgress,
  SearchResult
} from '@/types/search'
import type { ScanProgressPayload } from '@/types/license'
import { parseLanguages } from '@/utils/api'
import { useSearch } from './useSearch'
import { useClustering } from './useClustering'
import { useToast } from './useToast'
import { usePreferences } from './usePreferences'
import { t } from '@/i18n'

// ---------- 模块级共享状态 ----------
// P1-5 / 债单 A7：folderPath / scanMode / ocrLanguages 改从 usePreferences 取，
// 复用其 watch 持久化（localStorage），重开 App 自动回填，不再每次重新粘贴路径。
const { folderPath, scanMode, ocrLang: ocrLanguages } = usePreferences()
const enableOcr = ref(false)

const isScanning = ref(false)
const scanMsg = ref('')
const currentStatus = ref('')
const currentFile = ref('')
const scanProgress = ref<ScanProgress>({ current: 0, total: 0 })
/** P1-9：扫描开始时间戳，用于计算 ETA */
const scanStartTime = ref(0)
/** P1-9：已用时间（秒） */
const scanElapsed = computed(() => {
  if (!isScanning.value || scanStartTime.value === 0) return 0
  return Math.floor((Date.now() - scanStartTime.value) / 1000)
})
/** P1-9：预计剩余时间（秒），按当前速度估算 */
const scanEta = computed(() => {
  const p = scanProgress.value
  if (p.total <= 0 || p.current <= 0) return 0
  const elapsed = scanElapsed.value
  if (elapsed < 1) return 0
  const speed = p.current / elapsed
  if (speed <= 0) return 0
  return Math.ceil((p.total - p.current) / speed)
})

const incomingFiles = ref<SearchResult[]>([])
const showIncomingBanner = ref(false)
const incomingCount = computed(() => incomingFiles.value.length)

// 幽灵清理：预览结果、对话框开关、动作执行中的忙碌态
const ghostItems = ref<GhostItem[]>([])
const ghostDialogOpen = ref(false)
const ghostBusy = ref(false)
const ghostMsg = ref('')

// 差异报告：只读感知的结果、对话框开关、动作执行中的忙碌态
const diffReport = ref<DiffReport | null>(null)
const diffDialogOpen = ref(false)
const diffBusy = ref(false)
const diffMsg = ref('')

// 扫描报告 / 活动记录
const scanReports = ref<BatchReport[]>([])
const reportDialogOpen = ref(false)
const reportMsg = ref('')

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
    scanStartTime.value = Date.now() // P1-9：记录开始时间用于 ETA
    scanMsg.value = t('scanner.extracting')
    try {
      const count: number = await invoke('scan_folder', {
        folderPath: folderPath.value,
        scanMode: scanMode.value,
        enableOcr: enableOcr.value,
        ocrLanguages: parseLanguages(ocrLanguages.value)
      })

      scanMsg.value =
        count === 0
          ? t('scanner.scanNoNew')
          : t('scanner.memoryLoaded', { count })
    } catch (err) {
      scanMsg.value = t('scanner.failed', { err })
    } finally {
      isScanning.value = false
      // 仅在浏览模式下刷新列表，避免打断用户正在看的搜索结果
      if (!searchQuery.value && !isImageSearch.value && !isClusteringView.value) {
        await loadBrowsePage(currentPage.value)
      }
    }
  }

  /** P1-9：停止扫描（调后端 cancel_scan 命令）。 */
  async function stopScan() {
    const { push } = useToast()
    try {
      await invoke('cancel_scan')
      scanMsg.value = t('scanner.scanStopped')
      push(t('scanner.scanStopped'), 'info')
    } catch (err) {
      push(t('scanner.stopFailed', { err }), 'error')
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
    scanMsg.value = t('scanner.indexing', { count: selected.length })
    try {
      const count: number = await invoke('index_files', {
        filePaths: selected,
        enableOcr: enableOcr.value,
        ocrLanguages: parseLanguages(ocrLanguages.value)
      })
      scanMsg.value = t('scanner.indexed', { count })

      if (!searchQuery.value && !isImageSearch.value && !isClusteringView.value) {
        await loadBrowsePage(currentPage.value)
      }
    } catch (err) {
      scanMsg.value = t('scanner.indexFailed', { err })
    } finally {
      isScanning.value = false
    }
  }

  /** 打开扫描报告：读取 batches 表里的骨架与摘要（持久化，重启不丢） */
  async function openScanReports() {
    reportMsg.value = ''
    try {
      scanReports.value = await invoke<BatchReport[]>('list_scan_reports', { limit: 30 })
      reportDialogOpen.value = true
    } catch (err) {
      reportMsg.value = t('scanner.loadReportsFailed', { err })
    }
  }

  function closeScanReports() {
    reportDialogOpen.value = false
  }

  /**
   * 打开差异报告：一次轻量预扫描（只 stat，不跑模型），把磁盘与库的差异摆出来。
   * 只感知，不行动 —— 索引与否由用户在明细层勾选后决定。
   */
  async function openDiffReport() {
    diffMsg.value = ''
    if (!folderPath.value) {
      diffMsg.value = t('scanner.selectDirFirst')
      diffReport.value = null
      diffDialogOpen.value = true
      return
    }
    try {
      diffReport.value = await invoke<DiffReport>('scan_diff', {
        folderPath: folderPath.value
      })
      diffDialogOpen.value = true
    } catch (err) {
      diffMsg.value = t('scanner.diffFailed', { err })
    }
  }

  function closeDiffReport() {
    diffDialogOpen.value = false
  }

  /**
   * 把选中的条目送去索引。
   *  · 新增 / 待索引 → `index_files`（库里还没有，走首次入库流程）
   *  · 修改         → `reindex_files`（强制重新编码并整体替换旧帧行，
   *                    成功后才写 content_modified，失败写 reindex_failed 留在待索引组）
   */
  async function indexDiffSelection(paths: string[]) {
    if (paths.length === 0 || diffBusy.value) return
    diffBusy.value = true
    try {
      const all = diffReport.value?.items ?? []
      const kindOf = (p: string) => all.find((i) => i.path === p)?.kind
      const fresh = paths.filter((p) => kindOf(p) !== 'modified')
      const modified = paths.filter((p) => kindOf(p) === 'modified')

      const done = new Set<string>()
      const notes: string[] = []

      if (fresh.length) {
        const count: number = await invoke('index_files', {
          filePaths: fresh,
          enableOcr: enableOcr.value,
          ocrLanguages: parseLanguages(ocrLanguages.value)
        })
        notes.push(t('scanner.indexedCount', { count }))
        fresh.forEach((p) => done.add(p))
      }

      if (modified.length) {
        const result = await invoke<ReindexResult>('reindex_files', {
          paths: modified,
          enableOcr: enableOcr.value,
          ocrLanguages: parseLanguages(ocrLanguages.value)
        })
        notes.push(
          t('scanner.reindexed', { count: result?.succeeded?.length ?? 0, frames: result?.replaced_frames ?? 0 })
        )
        ;(result?.succeeded ?? []).forEach((p) => done.add(p))
        if (result?.failed?.length) {
          notes.push(t('scanner.reindexFailedCount', { count: result.failed.length }))
        }
      }

      diffMsg.value = t('scanner.notes', { notes: notes.join(', ') })

      // 就地刷新报告：成功处理的条目不再出现在清单里，失败的留着待重试
      if (diffReport.value) {
        diffReport.value.items = diffReport.value.items.filter((i) => !done.has(i.path))
        recountDiff()
      }

      if (!searchQuery.value && !isImageSearch.value && !isClusteringView.value) {
        await loadBrowsePage(currentPage.value)
      } else {
        await performSearch()
      }
    } catch (err) {
      diffMsg.value = t('scanner.indexFailed', { err })
    } finally {
      diffBusy.value = false
    }
  }

  /** 标记失效：交给幽灵清理那套逻辑（向量保留，随时可回库） */
  async function markDeadDiffSelection(paths: string[]) {
    if (paths.length === 0 || diffBusy.value) return
    diffBusy.value = true
    try {
      const result: GhostActionResult = await invoke('apply_ghost_action', {
        paths,
        action: 'mark_dead'
      })
      const affected = result?.affected_paths ?? []
      diffMsg.value = t('scanner.markedDead', { count: affected.length })

      const done = new Set(affected)
      if (diffReport.value) {
        diffReport.value.items = diffReport.value.items.filter((i) => !done.has(i.path))
        recountDiff()
      }
    } catch (err) {
      diffMsg.value = t('scanner.actionFailed', { err })
    } finally {
      diffBusy.value = false
    }
  }

  /**
   * 忽略：**刻意什么都不告诉后端**。
   * 忽略只作用于本次批次——不产生事件、不更新 observed_*。
   * 下次扫描若该文件仍未变化，它会再次出现，用户可以再选一次忽略，
   * 或用「加入索引 / 标记失效」来真正结束这个状态。
   */
  function ignoreDiffSelection(paths: string[]) {
    const done = new Set(paths)
    if (diffReport.value) {
      diffReport.value.items = diffReport.value.items.filter((i) => !done.has(i.path))
      recountDiff()
    }
    diffMsg.value = t('scanner.ignored', { count: paths.length })
  }

  function recountDiff() {
    const items = diffReport.value?.items ?? []
    const c = (kind: string) => items.filter((i) => i.kind === kind).length
    if (diffReport.value) {
      diffReport.value.new_count = c('new')
      diffReport.value.modified_count = c('modified')
      diffReport.value.missing_count = c('missing')
      diffReport.value.unindexed_count = c('unindexed')
    }
  }

  /**
   * 打开幽灵清理对话框：只做一次只读预览，不改动任何记录。
   * 「系统有感知，行动由用户触发」——发现问题是系统的活，怎么处理是用户的活。
   */
  async function openGhostCleanup() {
    ghostMsg.value = ''
    try {
      ghostItems.value = await invoke<GhostItem[]>('preview_ghosts')
      ghostDialogOpen.value = true
    } catch (err) {
      ghostMsg.value = t('scanner.ghostPreviewFailed', { err })
    }
  }

  function closeGhostCleanup() {
    ghostDialogOpen.value = false
  }

  /**
   * 对勾选的路径执行动作。动作由用户在预览后明确选择，此处只负责执行与回显。
   */
  async function applyGhostAction(action: 'mark_dead' | 'restore' | 'purge', paths: string[]) {
    if (paths.length === 0 || ghostBusy.value) return

    ghostBusy.value = true
    try {
      const result: GhostActionResult = await invoke('apply_ghost_action', {
        paths,
        action
      })

      const affected = result?.affected_paths ?? []
      const skipped = result?.skipped_paths?.length ?? 0
      ghostMsg.value =
        affected.length === 0
          ? t('scanner.nothingProcessed', { count: skipped })
          : t('scanner.processed', { count: affected.length, frames: result?.affected_frames ?? 0 }) +
            (skipped ? ' · ' + t('scanner.skipped', { count: skipped }) : '')

      // 就地更新预览：被处理的条目从清单里移除，用户不必重新打开对话框
      const done = new Set(affected)
      ghostItems.value = ghostItems.value.filter((i) => !done.has(i.path))

      // 同步当前结果列表
      results.value = results.value.filter((r) => !done.has(r.path))
      totalResults.value = Math.max(0, totalResults.value - affected.length)

      if (!searchQuery.value && !isImageSearch.value && !isClusteringView.value) {
        await loadBrowsePage(currentPage.value)
      } else {
        await performSearch()
      }
    } catch (err) {
      ghostMsg.value = t('scanner.actionFailed', { err })
    } finally {
      ghostBusy.value = false
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
            matched_tags: [t('scanner.freshIndex')],
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
    scanElapsed,
    scanEta,
    stopScan,
    incomingFiles,
    showIncomingBanner,
    incomingCount,
    ghostItems,
    ghostDialogOpen,
    ghostBusy,
    ghostMsg,
    diffReport,
    diffDialogOpen,
    diffBusy,
    diffMsg,
    scanReports,
    reportDialogOpen,
    reportMsg,
    // actions
    selectFolder,
    openScanReports,
    closeScanReports,
    openDiffReport,
    closeDiffReport,
    indexDiffSelection,
    markDeadDiffSelection,
    ignoreDiffSelection,
    startScan,
    selectAndIndexFiles,
    openGhostCleanup,
    closeGhostCleanup,
    applyGhostAction,
    acceptIncomingFiles,
    dismissIncomingBanner
  }
}
