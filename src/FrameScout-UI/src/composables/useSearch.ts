/**
 * useSearch — 多模态搜索、浏览分页、以图搜图、防竞态。
 *
 * 依赖方向上的叶子节点：只依赖 utils，不依赖其他 composable。
 * 状态声明在模块级 => 单例。App.vue 与各子组件调用 useSearch() 拿到的是同一份状态。
 */
import { ref, computed, onUnmounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { SearchResult, PagedResponse, TextSearchHit, TextEntryHit, SearchMode } from '@/types/search'
import { isVideo } from '@/utils/media'
import { getNumericScore, getSearchNumericScore, LOW_SCORE_THRESHOLD } from '@/utils/score'
import { armVideoTimeout, clearAllVideoTimers } from '@/utils/videoTimers'
import { usePreferences } from './usePreferences'
import { t } from '@/i18n'

// ---------- 模块级共享状态 ----------
const searchQuery = ref('')
const isImageSearch = ref(false)
const searchImagePath = ref('')

// 默认模式为 exact（纯文字），故图片（视觉）默认不勾
const s_filename = ref(true)
const s_note = ref(true)
const s_ocr = ref(true)
const s_vector = ref(false)
// 纯文本目标（BGE-M3 模式下可勾选）：查 text_entries，独立于媒体命中
const s_text = ref(false)

// 纯文本检索结果（独立于媒体结果，命中展示的是内容本身）
const textEntryResults = ref<TextEntryHit[]>([])
// 纯文本结果的分页：与媒体结果是两个独立结果集，故页码互不干扰
const textEntryPage = ref(1)
const textEntryTotal = ref(0)
// 纯文本录入对话框状态
const textEntryDialogOpen = ref(false)
const textEntryInput = ref('')
const textEntrySource = ref('')

/**
 * 搜索模式（三条检索公路，绝不混算）：
 *  - exact    ：字符串精确匹配（文件名 / 笔记 / OCR）
 *  - semantic ：SigLIP 视觉语义（以文搜图）+ 可选的文字模糊匹配
 *  - bge      ：BGE-M3 文本语义（OCR 文本的 dense 检索）+ 可选的纯文本条目
 */
const searchMode = ref<SearchMode>('exact')

// 切换模式时应用该模式的默认目标。
//
// **P1-12 修复（第三轮「切换模式会静默重写用户勾选」）**：
//   旧实现用 `flush: 'sync'` 直接重置 5 个目标布尔值，用户精心勾选的组合
//   在切换模式时被静默抹掉，且无提示——违反《总纲》设计哲学第二条
//   「用户输入即意图，FrameScout 不做隐式改写」。
//   新实现：仅在「用户未手动改过目标」时才应用默认值；一旦用户改过任何目标，
//   切换模式不再自动重写，保留用户选择。
let userTouchedTargets = false
let modeSwitchInProgress = false // 区分模式切换触发的目标变更 vs 用户手动变更
watch(
  searchMode,
  (mode) => {
    // 用户已手动调整过目标 → 不再静默重写
    if (userTouchedTargets) return

    modeSwitchInProgress = true
    if (mode === 'exact') {
      s_filename.value = true
      s_note.value = true
      s_ocr.value = true
      s_vector.value = false
      s_text.value = false
    } else if (mode === 'semantic') {
      s_filename.value = false
      s_note.value = false
      s_ocr.value = false
      s_vector.value = true
      s_text.value = false
    } else {
      s_filename.value = false
      s_note.value = false
      s_ocr.value = true
      s_vector.value = false
      s_text.value = false
    }
    // nextTick 后恢复标志，使后续用户变更能被正确追踪
    setTimeout(() => { modeSwitchInProgress = false }, 0)
  }
)

// P1-12：用户手动改目标时标记，之后切换模式不再静默重写
watch([s_filename, s_note, s_ocr, s_vector, s_text], () => {
  if (!modeSwitchInProgress) {
    userTouchedTargets = true
  }
})

const results = ref<SearchResult[]>([])
const currentPage = ref(1)
// P1-5 / 债单 A7：pageSize 改从 usePreferences 取，重开 App 保持上次每页条数
const { pageSize, addSearchHistory } = usePreferences()
const jumpPage = ref(1)
const totalResults = ref(0)
const totalPages = computed(() => Math.max(1, Math.ceil(totalResults.value / pageSize.value)))

/** 纯文本结果的页数（与媒体结果是两个独立结果集，页数分开算） */
const textEntryTotalPages = computed(() =>
  Math.max(1, Math.ceil(textEntryTotal.value / pageSize.value))
)

const isShowAllMode = ref(false)
const searchMsg = ref('')

/** P1-8：搜索进行中标志（旧实现无此 ref，界面静止时用户不知是在算还是卡死） */
const isSearching = ref(false)

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

  // 低置信度图片默认折叠（视频卡片不折叠）。方案 C：按来源选正确的映射。
  items.forEach((item) => {
    let num: number
    if (item.scoreKind === 'bge') num = getNumericScore(item.score, false, 'bge')
    else if (isImg) num = getNumericScore(item.score, true)
    else num = getSearchNumericScore(item)
    if (!isVideo(item.path) && num < LOW_SCORE_THRESHOLD) {
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
      searchMsg.value = t('searcher.loadFilesFailed', { err })
    }
  }

  async function performSearch() {
    isShowAllMode.value = false
    const currentId = ++searchRequestId
    isSearching.value = true // P1-8：搜索进行中标志

    if (!searchQuery.value && !isImageSearch.value) {
      results.value = []
      isSearching.value = false
      totalResults.value = 0
      return
    }
    searchMsg.value = ''

    // P0-3：搜索历史——记录用户真正敲下的查询词（以图搜图无查询词，不记录）
    if (!isImageSearch.value && searchQuery.value.trim()) {
      addSearchHistory(searchQuery.value.trim())
    }

    // 新一轮搜索：清掉上一轮残留的视频超时计时器与纯文本结果
    clearAllVideoTimers()
    textEntryResults.value = []

    try {
      let response: PagedResponse<SearchResult>

      if (isImageSearch.value) {
        // 以图搜图：视觉向量
        response = await invoke('search_by_image', {
          imagePath: searchImagePath.value,
          page: currentPage.value,
          limit: pageSize.value
        })
      } else if (searchMode.value === 'bge') {
        // BGE-M3 文本语义，两条独立公路：
        //   · OCR 目标 → search_text（媒体命中，映射成 SearchResult）
        //   · 纯文本目标 → search_text_entries（独立条目，存 textEntryResults）
        let mediaItems: SearchResult[] = []
        let mediaTotal = 0

        if (s_ocr.value) {
          const raw = await invoke<PagedResponse<TextSearchHit>>('search_text', {
            text: searchQuery.value,
            page: currentPage.value,
            limit: pageSize.value
          })
          mediaItems = raw.items.map(
            (h): SearchResult => ({
              path: h.path,
              timestamp: h.timestamp,
              score: h.similarity,
              matched_tags: [t('searcher.bgeM3')],
              ocr_text: h.ocr_text,
              user_note: h.user_note,
              index_time: h.index_time,
              scoreKind: 'bge'
            })
          )
          mediaTotal = raw.total_count
        }

        if (s_text.value) {
          // 新一轮搜索从第一页开始（纯文本是独立结果集，有自己的页码）
          await loadTextEntryPage(1)
        }

        response = { items: mediaItems, total_count: mediaTotal }
      } else if (searchMode.value === 'semantic') {
        // 模糊（SigLIP 视觉语义 + 可选文字模糊匹配）：默认只勾图片（纯视觉语义），
        // 但用户可勾选文件名 / 笔记 / OCR，把文字 contains 也混进来（文件管理器式模糊搜索）。
        // P2-2 / 债单 A1：改走 search_unified，单遍扫描并带回 matches（得分来源）+ ocr_lines（红框）。
        response = await invoke('search_unified', {
          text: searchQuery.value,
          page: currentPage.value,
          limit: pageSize.value,
          useVector: s_vector.value,
          useOcr: s_ocr.value,
          useNote: s_note.value,
          useFilename: s_filename.value
        })
      } else {
        // 精确：字符串匹配（文件名 / 笔记 / OCR），不开视觉向量
        response = await invoke('search_unified', {
          text: searchQuery.value,
          page: currentPage.value,
          limit: pageSize.value,
          useVector: false,
          useOcr: s_ocr.value,
          useNote: s_note.value,
          useFilename: s_filename.value
        })
      }

      // 已有更新的请求发出，本次响应作废
      if (currentId !== searchRequestId) return

      results.value = normalizeResults(response.items, isImageSearch.value)
      totalResults.value = response.total_count
    } catch (err) {
      if (currentId === searchRequestId) {
        searchMsg.value = t('searcher.error', { err })
      }
    } finally {
      if (currentId === searchRequestId) {
        isSearching.value = false
      }
    }
  }

  /**
   * 纯文本结果翻页：只重搜文本条目，**不惊动媒体结果**。
   * 纯文本与媒体是两个独立结果集，各自的页码互不干扰。
   */
  async function loadTextEntryPage(page: number) {
    if (!searchQuery.value || searchMode.value !== 'bge' || !s_text.value) return
    textEntryPage.value = page
    try {
      const res = await invoke<PagedResponse<TextEntryHit>>('search_text_entries', {
        text: searchQuery.value,
        page,
        limit: pageSize.value
      })
      textEntryResults.value = res.items
      textEntryTotal.value = res.total_count
    } catch (err) {
      searchMsg.value = t('searcher.textSearchFailed', { err })
    }
  }

  async function changeTextEntryPage(delta: number) {
    const next = textEntryPage.value + delta
    if (next < 1 || next > textEntryTotalPages.value) return
    await loadTextEntryPage(next)
  }

  async function jumpTextEntryPage(page: number) {
    let target = page
    if (target < 1) target = 1
    if (target > textEntryTotalPages.value) target = textEntryTotalPages.value
    await loadTextEntryPage(target)
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
      searchQuery.value = t('searcher.visualSeeking')
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
    // P1-8/P9：Show All 也加竞态守卫（旧实现未保护，连点会旧响应覆盖新结果）
    const currentId = ++searchRequestId
    isSearching.value = true
    try {
      // 先退出搜索/图像态，确保 normalize 用的是文本阈值
      searchQuery.value = ''
      isImageSearch.value = false
      searchImagePath.value = ''

      const allItems: SearchResult[] = await invoke('get_all_files')
      if (currentId !== searchRequestId) return // 已有更新的请求
      results.value = normalizeResults(allItems, false)
      totalResults.value = allItems.length
      isShowAllMode.value = true
    } catch (err) {
      if (currentId === searchRequestId) {
        searchMsg.value = t('searcher.loadAllFailed', { err })
      }
    } finally {
      if (currentId === searchRequestId) {
        isSearching.value = false
      }
    }
  }

  function exitShowAllMode() {
    isShowAllMode.value = false
    currentPage.value = 1
    jumpPage.value = 1
    loadBrowsePage(1)
  }

  /** 纯文本入库：把用户输入的一段文本交给 worker 编码后落 `text_entries`。 */
  async function insertTextEntry() {
    const text = textEntryInput.value.trim()
    if (!text) return
    try {
      await invoke<string>('insert_text_entry', {
        text,
        sourceUri: textEntrySource.value.trim() || 'user_note'
      })
      textEntryInput.value = ''
      textEntrySource.value = ''
      textEntryDialogOpen.value = false
      searchMsg.value = t('searcher.textAdded')
    } catch (err) {
      searchMsg.value = t('searcher.insertFailed', { err })
    }
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
    s_text,
    searchMode,
    results,
    textEntryResults,
    textEntryPage,
    textEntryTotal,
    textEntryTotalPages,
    textEntryDialogOpen,
    textEntryInput,
    textEntrySource,
    currentPage,
    pageSize,
    jumpPage,
    totalResults,
    totalPages,
    isShowAllMode,
    searchMsg,
    isSearching,
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
    exitShowAllMode,
    insertTextEntry,
    changeTextEntryPage,
    jumpTextEntryPage
  }
}
