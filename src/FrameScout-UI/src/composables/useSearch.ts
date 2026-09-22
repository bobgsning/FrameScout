/**
 * useSearch — 多模态搜索、浏览分页、以图搜图、防竞态。
 *
 * 依赖方向上的叶子节点：只依赖 utils，不依赖其他 composable。
 * 状态声明在模块级 => 单例。App.vue 与各子组件调用 useSearch() 拿到的是同一份状态。
 */
import { ref, computed, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { SearchResult, PagedResponse } from '@/types/search'
import { isVideo } from '@/utils/media'
import { getNumericScore, LOW_SCORE_THRESHOLD } from '@/utils/score'
import { armVideoTimeout, clearAllVideoTimers } from '@/utils/videoTimers'

// ---------- 模块级共享状态 ----------
const searchQuery = ref('')
const isImageSearch = ref(false)
const searchImagePath = ref('')

const s_filename = ref(true)
const s_note = ref(true)
const s_ocr = ref(true)
const s_vector = ref(true)

const results = ref<SearchResult[]>([])
const currentPage = ref(1)
const pageSize = ref(8)
const jumpPage = ref(1)
const totalResults = ref(0)
const totalPages = computed(() => Math.max(1, Math.ceil(totalResults.value / pageSize.value)))

const isShowAllMode = ref(false)
const searchMsg = ref('')

/** 进入聚类视图前暂存当前结果，退出时原样恢复 */
const fullResultsCache = ref<SearchResult[]>([])

/** 防竞态：每次发起搜索自增，回来时对不上号的结果直接丢弃 */
let searchRequestId = 0

/** 守卫：onUnmounted 兜底清理只注册一次（挂在首次调用的 App.vue 上）。
 *  否则 OcrPanel / TopActionBar 等子组件经 useScanner -> useSearch 调用时，
 *  会把该 hook 挂到子组件，导致结果卡片翻页/卸载时误清空其它视频的加载超时计时器。
 *  onUnmounted 末尾将其重置为 false，以便 HMR 后重新挂载时能再次订阅。 */
let lifecycleBound = false

/** 把后端返回的裸结果规整成前端用的形态：
 *  - 补齐 UI 用的可选字段（isMissing/expandOcr/ocrRunning/…）
 *  - 给视频武装 25s 加载超时（超时则标记 isMissing）
 *  - 给低置信度图片设置 collapsedLowScore（默认折叠）
 *  导出供 useSmartFolders.applySmartFolder 复用，保持各入口行为一致。 */
export function normalizeResults(items: SearchResult[], isImg: boolean): SearchResult[] {
  items.forEach((item) => {
    item.isMissing = false
    item.expandOcr = false
    item.ocrLangInput = ''
    item.ocr_text = item.ocr_text || ''
    item.ocrRunning = false
    item.matched_tags = item.matched_tags || []

    if (isVideo(item.path)) {
      armVideoTimeout(item.path, () => {
        item.isMissing = true
      })
    }
  })

  // 低置信度图片默认折叠（视频卡片不折叠）
  items.forEach((item) => {
    if (!isVideo(item.path) && getNumericScore(item.score, isImg) < LOW_SCORE_THRESHOLD) {
      item.collapsedLowScore = true
    }
  })

  return items
}

export function useSearch() {
  /** 浏览模式：既没有文本查询，也不是以图搜图 */
  const isBrowsingMode = computed(() => !searchQuery.value && !isImageSearch.value)

  async function loadBrowsePage(page: number) {
    try {
      const response: PagedResponse<SearchResult> = await invoke('list_all_files', {
        page,
        limit: pageSize.value
      })

      // 目标页已空（例如刚 purge 过），回落到最后一个有效页
      if (response.items.length === 0 && page > 1) {
        const lastPage = Math.max(1, Math.ceil(response.total_count / pageSize.value))
        if (lastPage !== page) {
          currentPage.value = lastPage
          jumpPage.value = lastPage
          return loadBrowsePage(lastPage)
        }
      }

      results.value = normalizeResults(response.items, false)
      totalResults.value = response.total_count
      fullResultsCache.value = [...results.value]
    } catch (err) {
      searchMsg.value = `⚠️ Failed to load files: ${err}`
    }
  }

  async function performSearch() {
    isShowAllMode.value = false
    const currentId = ++searchRequestId

    if (!searchQuery.value && !isImageSearch.value) {
      results.value = []
      totalResults.value = 0
      return
    }
    searchMsg.value = ''

    // 新一轮搜索：清掉上一轮残留的视频超时计时器
    clearAllVideoTimers()

    try {
      const response: PagedResponse<SearchResult> = isImageSearch.value
        ? await invoke('search_by_image', {
            imagePath: searchImagePath.value,
            page: currentPage.value,
            limit: pageSize.value
          })
        : await invoke('search_images', {
            text: searchQuery.value,
            page: currentPage.value,
            limit: pageSize.value,
            useVector: s_vector.value,
            useOcr: s_ocr.value,
            useNote: s_note.value,
            useFilename: s_filename.value
          })

      // 已有更新的请求发出，本次响应作废
      if (currentId !== searchRequestId) return

      results.value = normalizeResults(response.items, isImageSearch.value)
      totalResults.value = response.total_count
    } catch (err) {
      if (currentId === searchRequestId) {
        searchMsg.value = `⚠️ Error: ${err}`
      }
    }
  }

  async function resetAndSearch() {
    isImageSearch.value = false
    currentPage.value = 1
    jumpPage.value = 1
    await performSearch()
  }

  async function searchByImageAction() {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Images', extensions: ['png', 'jpeg', 'jpg', 'webp'] }]
    })
    if (selected) {
      searchImagePath.value = selected as string
      isImageSearch.value = true
      searchQuery.value = `[Visual Mode] Seeking semantic resonance...`
      currentPage.value = 1
      jumpPage.value = 1
      await performSearch()
    }
  }

  function clearImageSearch() {
    if (!isImageSearch.value) return
    isImageSearch.value = false
    searchImagePath.value = ''
    searchQuery.value = ''
    results.value = []
    totalResults.value = 0
  }

  async function changePage(delta: number) {
    const newPage = currentPage.value + delta
    if (newPage < 1 || newPage > totalPages.value) return

    currentPage.value = newPage
    jumpPage.value = currentPage.value

    // FIX: 原实现在浏览模式下翻页会走 performSearch，导致结果被清空
    // （performSearch 在无查询词时会直接 return 空结果）。浏览模式应走分页接口。
    if (isBrowsingMode.value) {
      await loadBrowsePage(currentPage.value)
    } else {
      await performSearch()
    }
  }

  async function doJump(page?: number) {
    // 分页条会把输入框的值直接传进来；不传时沿用 jumpPage
    if (page !== undefined) jumpPage.value = page

    if (jumpPage.value < 1) jumpPage.value = 1
    if (jumpPage.value > totalPages.value) jumpPage.value = totalPages.value

    currentPage.value = jumpPage.value

    if (isBrowsingMode.value) {
      await loadBrowsePage(currentPage.value)
    } else {
      await performSearch()
    }
  }

  async function showAllFiles() {
    try {
      // 先退出搜索/图像态，确保 normalize 用的是文本阈值
      searchQuery.value = ''
      isImageSearch.value = false
      searchImagePath.value = ''

      const allItems: SearchResult[] = await invoke('get_all_files')
      results.value = normalizeResults(allItems, false)
      totalResults.value = allItems.length
      isShowAllMode.value = true
    } catch (err) {
      searchMsg.value = `⚠️ Failed to load all files: ${err}`
    }
  }

  function exitShowAllMode() {
    isShowAllMode.value = false
    currentPage.value = 1
    jumpPage.value = 1
    loadBrowsePage(1)
  }

  // 兜底清理：仅在整个应用（App.vue 首次调用 useSearch）卸载时全量清理。
  // 必须用 lifecycleBound 守卫，避免被子组件调用时把 hook 挂到子组件实例上。
  // 末尾重置 lifecycleBound，便于 HMR 后重新挂载时再次订阅。
  if (!lifecycleBound) {
    lifecycleBound = true
    onUnmounted(() => {
      clearAllVideoTimers()
      lifecycleBound = false
    })
  }

  return {
    // state
    searchQuery,
    isImageSearch,
    searchImagePath,
    s_filename,
    s_note,
    s_ocr,
    s_vector,
    results,
    currentPage,
    pageSize,
    jumpPage,
    totalResults,
    totalPages,
    isShowAllMode,
    searchMsg,
    fullResultsCache,
    isBrowsingMode,
    // actions
    performSearch,
    resetAndSearch,
    searchByImageAction,
    clearImageSearch,
    changePage,
    doJump,
    loadBrowsePage,
    showAllFiles,
    exitShowAllMode
  }
}
