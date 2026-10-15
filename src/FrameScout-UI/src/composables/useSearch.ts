/**
 * useSearch — 逻辑搜索（第七轮重构：多条件行 + 分层求值；第八轮核查修复）。
 *
 * 第七轮把「一门要背的符号方言」（`-`/`@`/`|`）换成「一张可点、可删、看得懂的条件表」：
 *   - 搜索框（searchQuery）只做纯关键词，不再解析任何语法（后端也已同步移除 `-` 排除词）；
 *   - 条件行（conditions）承载类型/值/作用域/阈值，组级「全部满足/任一满足」（groupOp）；
 *   - 求值分层：时间/目录/批次/类型 = L1 后端过滤；单一主条件 = L2 单次检索；
 *     仅「任一满足 / 排除 / 分条件阈值 / 非主条件排序」才 L3 拆叶子；
 *   - 主条件排序：分数是「某个具体条件对这张图的匹配度」，不编综合分；
 *   - 叶子缓存：翻页只 slice，不重跑子查询。
 *
 * 第八轮核查修复（本轮）：
 *   1. **以图搜图恢复**：重构时 `if (isImageSearch) → search_by_image` 分支被删掉未补，
 *      点「以图搜图」必然 0 结果。现把横幅里的图转成隐式 `seed` 条件，走同一条求值链。
 *   2. **混合检索分数**：visual 条件改用 `getSearchNumericScore`（先看视觉余弦、再看字面命中），
 *      不再把「命中通道数 1/2/3」当成余弦映射成 99.9~100 分。
 *   3. **file_type 条件**：后端已支持 file_types 过滤，此前是 UI 不可达的死类型，现接上。
 *   4. **排序选择器**：可切换「按主条件 / 按入库时间 / 按文件名」，非主条件排序走 L3 全量以保证全局有序。
 *   5. **搜索状态可恢复**：真正搜过一次才落盘，重开 App 恢复条件但**不自动搜索**（不替用户执行）。
 *
 * 状态声明在模块级 => 单例。App.vue 与各子组件调用 useSearch() 拿到的是同一份状态。
 */
import { ref, computed, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type {
  SearchResult,
  PagedResponse,
  TextSearchHit,
  TextEntryHit,
  ConditionRow,
  GroupOp,
  ConditionType,
  ConditionTarget
} from '@/types/search'
import { isVideo } from '@/utils/media'
import { getNumericScore, getSearchNumericScore, getLowScoreThreshold } from '@/utils/score'
import { armVideoTimeout, clearAllVideoTimers } from '@/utils/videoTimers'
import { usePreferences } from './usePreferences'
import { t } from '@/i18n'

// ---------- 模块级共享状态 ----------
/** 快速输入框：纯关键词（不解析任何符号；`-` 也是普通字符） */
const searchQuery = ref('')
/** 条件行列表 */
const conditions = ref<ConditionRow[]>([])
/** 组级逻辑：全部满足（AND）/ 任一满足（OR），不嵌套 */
const groupOp = ref<GroupOp>('all')
/** 排序依据：主条件匹配度 / 入库时间 / 文件名。选后两项时走全量前端排序，保证跨页有序。 */
const sortBy = ref<SortBy>('main')

/** 快速输入框的通道选择（画面语义 / 文件名 / 笔记 / 文字识别 / 文本语义），默认字面三通道。
 *  「文本语义」（BGE）是实验性的、与其它四个互斥。 */
const quickTargets = ref({
  visual: false,
  filename: true,
  note: true,
  ocr: true,
  text: false
})

/** 第九轮：快速通道全部关闭 —— buildEffective 会回退到默认字面三通道。
 *  为避免「全取消却仍出结果」的困惑，UI 据此弹一个（暂不可关闭的）提示条。 */
const allQuickTargetsOff = computed(
  () =>
    !quickTargets.value.visual &&
    !quickTargets.value.filename &&
    !quickTargets.value.note &&
    !quickTargets.value.ocr &&
    !quickTargets.value.text
)

type QuickChannel = 'visual' | 'filename' | 'note' | 'ocr' | 'text'
export type SortBy = 'main' | 'index_time' | 'filename'

const isImageSearch = ref(false)
const searchImagePath = ref('')

// 纯文本检索结果（独立于媒体结果）
const textEntryResults = ref<TextEntryHit[]>([])
const textEntryPage = ref(1)
const textEntryTotal = ref(0)
// 纯文本录入对话框状态
const textEntryDialogOpen = ref(false)
const textEntryInput = ref('')
const textEntrySource = ref('')

const results = ref<SearchResult[]>([])
const currentPage = ref(1)
const { pageSize, addSearchHistory } = usePreferences()
const jumpPage = ref(1)
const totalResults = ref(0)
const totalPages = computed(() => Math.max(1, Math.ceil(totalResults.value / pageSize.value)))

const textEntryTotalPages = computed(() =>
  Math.max(1, Math.ceil(textEntryTotal.value / pageSize.value))
)

const isShowAllMode = ref(false)
const searchMsg = ref('')
const isSearching = ref(false)
const fullResultsCache = ref<SearchResult[]>([])

/** 主条件标签（供 UI 显示「按 ① XXX 的匹配度」） */
const mainConditionLabel = ref('')
/** 试算结果（不填结果区，只报命中数——「行动由用户触发」） */
const estimateMsg = ref('')
const isEstimating = ref(false)

/** 防竞态 */
let searchRequestId = 0
let lifecycleBound = false

// ---------- 叶子缓存（翻页不重跑） ----------
let cachedCombined: SearchResult[] | null = null
let cachedKey = ''

function cacheKeyOf(): string {
  return JSON.stringify({
    q: searchQuery.value,
    qt: quickTargets.value,
    conds: conditions.value.map((c) => ({
      type: c.type,
      value: c.value,
      targets: c.targets,
      minScore: c.minScore,
      timeFrom: c.timeFrom,
      timeTo: c.timeTo,
      pathPrefix: c.pathPrefix,
      batchId: c.batchId,
      seedPath: c.seedPath,
      fileTypes: c.fileTypes
    })),
    op: groupOp.value,
    sort: sortBy.value,
    img: isImageSearch.value ? searchImagePath.value : ''
  })
}

// ---------- 搜索状态的会话持久化（恢复但不自动执行） ----------
const KEY_SESSION = 'framescout_search_session'

interface SearchSession {
  q: string
  qt: Record<string, boolean>
  conds: Array<Partial<ConditionRow>>
  op: GroupOp
  sort: SortBy
  img?: string
}

/** 已落盘的上次搜索（用于「你上次在搜 X」提示），null = 没有可恢复的。 */
const restorableSession = ref<SearchSession | null>(loadSession())

function loadSession(): SearchSession | null {
  try {
    const raw = localStorage.getItem(KEY_SESSION)
    if (!raw) return null
    const o = JSON.parse(raw) as SearchSession
    const hasAny =
      (o.q && o.q.trim()) || (o.conds && o.conds.length > 0) || (o.img && o.img.trim())
    return hasAny ? o : null
  } catch {
    return null
  }
}

/** 真正搜过一次才落盘（浏览 / 清空条件不写），并在重开 App 时提示可恢复。 */
function persistSession() {
  try {
    const s: SearchSession = {
      q: searchQuery.value,
      qt: { ...quickTargets.value },
      conds: conditions.value.map((c) => ({ ...c })),
      op: groupOp.value,
      sort: sortBy.value,
      img: isImageSearch.value ? searchImagePath.value : ''
    }
    const hasAny = (s.q && s.q.trim()) || s.conds.length > 0 || (s.img && s.img.trim())
    if (!hasAny) {
      localStorage.removeItem(KEY_SESSION)
      return
    }
    localStorage.setItem(KEY_SESSION, JSON.stringify(s))
  } catch {
    /* localStorage 不可用时静默失败 */
  }
}

/** 把落盘的会话恢复到界面（**不自动搜索**）。 */
function restoreSession() {
  const s = restorableSession.value
  if (!s) return
  searchQuery.value = s.q || ''
  if (s.qt) quickTargets.value = { ...quickTargets.value, ...s.qt }
  conditions.value = (s.conds || []).map((c, i) => ({
    id: c.id || `c_restore_${i}`,
    type: (c.type || 'text') as ConditionType,
    ...c
  })) as ConditionRow[]
  groupOp.value = s.op === 'any' ? 'any' : 'all'
  sortBy.value = s.sort || 'main'
  if (s.img) {
    isImageSearch.value = true
    searchImagePath.value = s.img
  }
  dismissRestorable()
}

function dismissRestorable() {
  // 第八轮核查修复：原先只清内存 ref，localStorage 里的 KEY_SESSION 还在，
  // 关掉提示条后重开 App 又会弹出来。现同步落盘清除。
  restorableSession.value = null
  try {
    localStorage.removeItem(KEY_SESSION)
  } catch {
    /* localStorage 不可用时静默失败 */
  }
}

/** 会话提示条上的一句人话（「白天 + 2 个条件」/「以图搜图」/ 关键词）。 */
const restorableLabel = computed(() => {
  const s = restorableSession.value
  if (!s) return ''
  if (s.img) return t('searcher.restoredImage')
  const n = s.conds?.length || 0
  if ((s.q || '').trim() && n) return t('searcher.restoredQueryAndConds', { q: s.q.trim(), n })
  if (n) return t('searcher.restoredConds', { n })
  return (s.q || '').trim()
})

// ---------- 条件行的「序号」映射（供排序选择器「按 ① 的匹配度」） ----------
function conditionIndex(id: string): number {
  return conditions.value.findIndex((c) => c.id === id) + 1
}

/** 把后端返回的裸结果规整成前端用的形态（补齐 UI 字段、武装视频超时、低分折叠）。 */
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

  // 低置信度图片默认折叠（可整体关闭）。方案 C：按来源选正确的映射。
  const { lowScoreFoldEnabled } = usePreferences()
  if (lowScoreFoldEnabled.value) {
    const foldThreshold = getLowScoreThreshold()
    items.forEach((item) => {
      let num: number
      if (item.scoreKind === 'bge') num = getNumericScore(item.score, false, 'bge')
      else if (item.scoreKind === 'image') num = getNumericScore(item.score, true)
      else if (isImg) num = getNumericScore(item.score, true)
      else num = getSearchNumericScore(item)
      if (!isVideo(item.path) && num < foldThreshold) {
        item.collapsedLowScore = true
      }
    })
  }

  return items
}

// ---------- 求值引擎的类型 ----------
interface FilterParams {
  batchId: string | null
  rootPath: string | null
  indexTimeFrom: number | null
  indexTimeTo: number | null
  fileTypes: string[] | null
}

export function useSearch() {
  const isBrowsingMode = computed(
    () => !searchQuery.value && !isImageSearch.value && conditions.value.length === 0
  )

  // ---------- 条件行操作 ----------
  function newConditionId(): string {
    return `c${Date.now().toString(36)}${Math.random().toString(36).slice(2, 7)}`
  }

  function addCondition(type: ConditionType): ConditionRow {
    const base: ConditionRow = { id: newConditionId(), type }
    switch (type) {
      case 'text':
        base.value = ''
        base.targets = ['filename', 'note', 'ocr']
        break
      case 'exclude':
        base.value = ''
        base.targets = ['filename', 'note', 'ocr']
        break
      case 'visual':
        base.value = ''
        base.targets = []
        base.minScore = null
        break
      case 'text_semantic':
      case 'text_entry':
        base.value = ''
        break
      case 'time':
        base.timeFrom = null
        base.timeTo = null
        break
      case 'path':
        base.pathPrefix = ''
        break
      case 'batch':
        base.batchId = ''
        break
      case 'file_type':
        base.fileTypes = []
        break
      case 'seed':
        base.seedPath = ''
        base.minScore = null
        break
    }
    // BGE 是实验性、互斥的：添加 BGE 条件时清空其它条件
    if (type === 'text_semantic' || type === 'text_entry') {
      conditions.value = [base]
    } else {
      // 添加非 BGE 条件时，若已有 BGE 条件，先清空（保持互斥）
      const hasBge = conditions.value.some((c) => c.type === 'text_semantic' || c.type === 'text_entry')
      if (hasBge) conditions.value = []
      conditions.value.push(base)
    }
    invalidateCache()
    return base
  }

  function removeCondition(id: string) {
    conditions.value = conditions.value.filter((c) => c.id !== id)
    invalidateCache()
  }

  function updateCondition(id: string, patch: Partial<ConditionRow>) {
    const cond = conditions.value.find((c) => c.id === id)
    if (!cond) return
    Object.assign(cond, patch)
    invalidateCache()
  }

  function clearConditions() {
    conditions.value = []
    invalidateCache()
  }

  function setGroupOp(op: GroupOp) {
    groupOp.value = op
    invalidateCache()
  }

  function setSortBy(s: SortBy) {
    sortBy.value = s
    invalidateCache()
  }

  function invalidateCache() {
    cachedCombined = null
    cachedKey = ''
  }

  /** 设置快速搜索通道，处理「文本语义」与其它四个的互斥（选 BGE 清空其它，选其它清空 BGE）。 */
  function setQuickTarget(key: QuickChannel, value: boolean) {
    if (key === 'text') {
      if (value) {
        quickTargets.value.visual = false
        quickTargets.value.filename = false
        quickTargets.value.note = false
        quickTargets.value.ocr = false
      }
      quickTargets.value.text = value
    } else {
      quickTargets.value[key] = value
      if (value) {
        quickTargets.value.text = false
      }
    }
    invalidateCache()
  }

  // ---------- 求值分层 ----------
  /** 从条件列表提取 L1 过滤参数（时间 / 目录 / 批次 / 类型），合并进后端调用。 */
  function buildFilterParams(conds: ConditionRow[]): FilterParams {
    let batchId: string | null = null
    let rootPath: string | null = null
    let indexTimeFrom: number | null = null
    let indexTimeTo: number | null = null
    let fileTypes: string[] | null = null
    for (const c of conds) {
      if (c.type === 'batch' && c.batchId) batchId = c.batchId
      else if (c.type === 'path' && c.pathPrefix) rootPath = c.pathPrefix
      else if (c.type === 'file_type' && c.fileTypes?.length) fileTypes = c.fileTypes
      else if (c.type === 'time') {
        if (c.timeFrom != null) indexTimeFrom = c.timeFrom
        if (c.timeTo != null) indexTimeTo = c.timeTo
      }
    }
    return { batchId, rootPath, indexTimeFrom, indexTimeTo, fileTypes }
  }

  /** 把一个条件映射成一次后端调用（叶子查询）。
   *  - `seed` → search_by_image（基准图 / 以图搜图）；
   *  - 其余 → search_unified，`useVector` 仅在 `visual` 条件为真。
   *  - `text` 的 targets 默认全部字面三通道；`visual` 的 targets 是「附加字面通道」（可空 = 纯视觉）。 */
  async function invokeLeaf(
    cond: ConditionRow,
    filter: FilterParams,
    limit: number,
    page = 1
  ): Promise<PagedResponse<SearchResult>> {
    if (cond.type === 'seed') {
      if (!cond.seedPath) return { items: [], total_count: 0 }
      const resp = await invoke<PagedResponse<SearchResult>>('search_by_image', {
        imagePath: cond.seedPath,
        page,
        limit,
        batchId: filter.batchId,
        rootPath: filter.rootPath,
        indexTimeFrom: filter.indexTimeFrom,
        indexTimeTo: filter.indexTimeTo,
        fileTypes: filter.fileTypes
      })
      // 第八轮核查修复：以图搜图返回的是「图-图余弦」，量纲与 SigLIP 文本-图余弦不同，
      // 必须打上 scoreKind='image'，前端才走正确映射 —— 否则落到文本阈值，
      // 余弦 0.5+ 全被压成 99.9%（卡片与排序同时失真）。横幅与 seed 条件行都走这里，统一打标。
      resp.items.forEach((it) => { it.scoreKind = 'image' })
      return resp
    }

    const useVector = cond.type === 'visual'
    const targets: ConditionTarget[] = cond.type === 'visual'
      ? (cond.targets ?? [])
      : (cond.targets?.length ? cond.targets : (['filename', 'note', 'ocr'] as ConditionTarget[]))
    const useOcr = targets.includes('ocr')
    const useNote = targets.includes('note')
    const useFilename = targets.includes('filename')

    return invoke<PagedResponse<SearchResult>>('search_unified', {
      text: cond.value || '',
      page,
      limit,
      useVector,
      useOcr,
      useNote,
      useFilename,
      batchId: filter.batchId,
      rootPath: filter.rootPath,
      indexTimeFrom: filter.indexTimeFrom,
      indexTimeTo: filter.indexTimeTo,
      fileTypes: filter.fileTypes
    })
  }

  /** 某个条件对某条命中的「匹配度」（人类可读分，用于主条件排序与阈值过滤）。
   *  第八轮修复：visual 条件改走 `getSearchNumericScore` —— 后端在纯字面命中时把
   *  `score` 设为「命中通道数(1~3)」，若当成余弦映射会得到 99.9~100 的假满分，
   *  把真正的视觉命中压到后面。`getSearchNumericScore` 会先看视觉余弦、再看字面命中。 */
  function condScore(cond: ConditionRow, it: SearchResult): number {
    if (cond.type === 'seed') return getNumericScore(it.score, true)
    if (it.scoreKind === 'bge') return getNumericScore(it.score, false, 'bge')
    return getSearchNumericScore(it)
  }

  /** 主条件 = 第一个正向条件（text / visual / seed）。 */
  function pickMain(positive: ConditionRow[]): ConditionRow | null {
    return positive[0] ?? null
  }

  /** 排序：默认按主条件匹配度；也可按入库时间 / 文件名（跨页全局有序）。 */
  function sortCombined(combined: SearchResult[], main: ConditionRow | null) {
    if (sortBy.value === 'index_time') {
      combined.sort(
        (a, b) => (b.index_time ?? 0) - (a.index_time ?? 0) || a.path.localeCompare(b.path)
      )
    } else if (sortBy.value === 'filename') {
      combined.sort((a, b) => a.path.localeCompare(b.path))
    } else if (main) {
      combined.sort((a, b) => condScore(main, b) - condScore(main, a) || a.path.localeCompare(b.path))
    }
  }

  /** 把组合后的全量结果排序并切片当前页。 */
  function applyCombined(combined: SearchResult[], main: ConditionRow | null) {
    sortCombined(combined, main)
    totalResults.value = combined.length
    const start = (currentPage.value - 1) * pageSize.value
    const pageItems = combined.slice(start, start + pageSize.value)
    results.value = normalizeResults(pageItems, false)
    fullResultsCache.value = [...results.value]
  }

  /** 组装「有效条件」：快速关键词（按通道选择转成隐式条件）+ 显式条件行 + 以图搜图基准图。 */
  function buildEffective(): ConditionRow[] {
    const effective: ConditionRow[] = []
    if (!isImageSearch.value && searchQuery.value.trim()) {
      const qt = quickTargets.value
      const literalTargets: ConditionTarget[] = []
      if (qt.filename) literalTargets.push('filename')
      if (qt.note) literalTargets.push('note')
      if (qt.ocr) literalTargets.push('ocr')
      if (qt.text) {
        effective.push({ id: 'quick', type: 'text_semantic', value: searchQuery.value.trim() })
      } else if (qt.visual) {
        effective.push({
          id: 'quick',
          type: 'visual',
          value: searchQuery.value.trim(),
          targets: literalTargets,
          minScore: null
        })
      } else {
        effective.push({
          id: 'quick',
          type: 'text',
          value: searchQuery.value.trim(),
          targets: literalTargets.length ? literalTargets : (['filename', 'note', 'ocr'] as ConditionTarget[])
        })
      }
    }
    effective.push(...conditions.value)

    // 第八轮修复：以图搜图横幅里的图 = 一个隐式「基准图」条件。
    // 重构时这条路径被整体删掉，导致点「以图搜图」必然 0 结果。
    // 用户若已自己加了基准图条件行，则以条件行为准，不重复注入。
    const hasSeedRow = effective.some((c) => c.type === 'seed')
    if (!hasSeedRow && isImageSearch.value && searchImagePath.value) {
      effective.unshift({
        id: 'seed_banner',
        type: 'seed',
        seedPath: searchImagePath.value,
        minScore: null
      })
    }
    return effective
  }

  async function performSearch() {
    isShowAllMode.value = false
    const currentId = ++searchRequestId
    isSearching.value = true

    // 浏览模式：既无关键词、无条件、也不是以图搜图
    if (isBrowsingMode.value) {
      results.value = []
      isSearching.value = false
      totalResults.value = 0
      return
    }

    searchMsg.value = ''

    // 搜索历史：记录用户真正敲下的关键词
    if (!isImageSearch.value && searchQuery.value.trim()) {
      addSearchHistory(searchQuery.value.trim())
    }

    clearAllVideoTimers()
    textEntryResults.value = []
    estimateMsg.value = ''

    const effective = buildEffective()

    // 命中缓存：翻页只 slice，不重跑子查询
    const key = cacheKeyOf()
    if (cachedCombined && cachedKey === key) {
      if (currentId !== searchRequestId) return
      applyCombined([...cachedCombined], pickMain(effective.filter(isPositive)))
      isSearching.value = false
      return
    }

    try {
      // BGE 互斥路径（实验性）
      const bge = effective.filter((c) => c.type === 'text_semantic' || c.type === 'text_entry')
      if (bge.length > 0) {
        await runBgeSearch(bge)
        if (currentId === searchRequestId) isSearching.value = false
        persistSession()
        return
      }

      const positive = effective.filter(isPositive)
      const exclude = effective.filter((c) => c.type === 'exclude')
      const filter = buildFilterParams(effective)

      if (positive.length === 0) {
        // 只有过滤/排除、没有正向条件：无法召回
        results.value = []
        totalResults.value = 0
        searchMsg.value = t('searcher.needPositive')
        if (currentId === searchRequestId) isSearching.value = false
        return
      }

      const main = pickMain(positive)
      mainConditionLabel.value = main ? (main.value || main.type) : ''

      // L2：单一正向主条件 + 无排除 + AND + 按主条件排序 → 单次检索（与普通搜索同速，后端分页）
      const needLeaf =
        positive.length > 1 ||
        exclude.length > 0 ||
        groupOp.value === 'any' ||
        sortBy.value !== 'main' ||
        positive.some((c) => c.minScore != null)
      if (!needLeaf) {
        const resp = await invokeLeaf(positive[0], filter, pageSize.value, currentPage.value)
        if (currentId !== searchRequestId) return
        results.value = normalizeResults(resp.items, false)
        totalResults.value = resp.total_count
        fullResultsCache.value = [...results.value]
        // 单条件走后端分页，翻页由 changePage → performSearch 重新按 currentPage 查；无需缓存
        cachedCombined = null
        cachedKey = ''
      } else {
        // L3：拆叶子 + 前端交/差/并 + 排序
        const combined = await runLeafSearch(positive, exclude, filter, main)
        if (currentId !== searchRequestId) return
        cachedCombined = combined
        cachedKey = key
        applyCombined(combined, main)
      }
      persistSession()
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

  function isPositive(c: ConditionRow): boolean {
    return c.type === 'text' || c.type === 'visual' || c.type === 'seed'
  }

  /** L3 拆叶子：每个正向/排除条件一次叶子查询，前端做交/差/并 + 排序 + 部分失败可见。 */
  async function runLeafSearch(
    positive: ConditionRow[],
    exclude: ConditionRow[],
    filter: FilterParams,
    main: ConditionRow | null
  ): Promise<SearchResult[]> {
    const leafMaps: Map<string, SearchResult>[] = []
    const failedLabels: string[] = []

    for (const cond of positive) {
      try {
        const resp = await invokeLeaf(cond, filter, 0)
        const map = new Map<string, SearchResult>()
        for (const it of resp.items) {
          if (cond.minScore != null && condScore(cond, it) < cond.minScore) continue
          map.set(it.path.toLowerCase(), it)
        }
        leafMaps.push(map)
      } catch {
        failedLabels.push(cond.value || cond.type)
      }
    }

    if (leafMaps.length === 0) {
      if (failedLabels.length) searchMsg.value = t('searcher.conditionFailed', { labels: failedLabels.join(', ') })
      return []
    }

    // 主条件叶子失败时（不在 leafMaps 里），退化为第一个可用叶子。
    const mainIdx = main ? positive.indexOf(main) : 0
    const mainMap = (mainIdx >= 0 ? leafMaps[mainIdx] : undefined) ?? leafMaps[0]

    const combined: SearchResult[] = []
    const seen = new Set<string>()

    if (groupOp.value === 'all') {
      for (const it of mainMap.values()) {
        const k = it.path.toLowerCase()
        const ok = leafMaps.every((m) => m === mainMap || m.has(k))
        if (ok && !seen.has(k)) {
          seen.add(k)
          combined.push(it)
        }
      }
    } else {
      // OR：主条件优先，其它条件补充新路径
      for (const it of mainMap.values()) {
        const k = it.path.toLowerCase()
        if (!seen.has(k)) {
          seen.add(k)
          combined.push(it)
        }
      }
      for (const m of leafMaps) {
        if (m === mainMap) continue
        for (const it of m.values()) {
          const k = it.path.toLowerCase()
          if (!seen.has(k)) {
            seen.add(k)
            combined.push(it)
          }
        }
      }
    }

    // 排除（NOT）：前端集合差
    if (exclude.length > 0) {
      const excludePaths = new Set<string>()
      for (const cond of exclude) {
        try {
          const resp = await invokeLeaf(cond, filter, 0)
          for (const it of resp.items) {
            if (cond.minScore != null && condScore(cond, it) < cond.minScore) continue
            excludePaths.add(it.path.toLowerCase())
          }
        } catch {
          failedLabels.push(cond.value || cond.type)
        }
      }
      for (let i = combined.length - 1; i >= 0; i--) {
        if (excludePaths.has(combined[i].path.toLowerCase())) {
          combined.splice(i, 1)
        }
      }
    }

    if (failedLabels.length) {
      searchMsg.value = t('searcher.conditionFailed', { labels: failedLabels.join(', ') })
    }

    return combined
  }

  /** 「试算」：只报命中数，不动结果区（设计 3.6 —— 不自动算，用户点了才算）。 */
  async function estimateHits() {
    if (isBrowsingMode.value) return
    const effective = buildEffective()
    const bge = effective.filter((c) => c.type === 'text_semantic' || c.type === 'text_entry')
    if (bge.length > 0) {
      estimateMsg.value = t('searcher.estimateBge')
      return
    }
    isEstimating.value = true
    try {
      const positive = effective.filter(isPositive)
      if (positive.length === 0) {
        estimateMsg.value = t('searcher.needPositive')
        return
      }
      const filter = buildFilterParams(effective)
      const exclude = effective.filter((c) => c.type === 'exclude')
      const needLeaf =
        positive.length > 1 || exclude.length > 0 || groupOp.value === 'any' || positive.some((c) => c.minScore != null)

      let n = 0
      if (!needLeaf) {
        const resp = await invokeLeaf(positive[0], filter, 1, 1)
        n = resp.total_count
      } else {
        const combined = await runLeafSearch(positive, exclude, filter, pickMain(positive))
        n = combined.length
      }
      estimateMsg.value = t('searcher.estimateResult', { n })
    } catch (err) {
      estimateMsg.value = t('searcher.error', { err })
    } finally {
      isEstimating.value = false
    }
  }

  /** BGE 互斥路径：text_semantic → search_text（媒体 OCR dense）；text_entry → search_text_entries。 */
  async function runBgeSearch(bge: ConditionRow[]) {
    const semantic = bge.find((c) => c.type === 'text_semantic')
    const entry = bge.find((c) => c.type === 'text_entry')

    let mediaItems: SearchResult[] = []
    let mediaTotal = 0
    if (semantic && semantic.value?.trim()) {
      const raw = await invoke<PagedResponse<TextSearchHit>>('search_text', {
        text: semantic.value.trim(),
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

    if (entry && entry.value?.trim()) {
      await loadTextEntryPage(1, entry.value.trim())
    }

    results.value = normalizeResults(mediaItems, false)
    totalResults.value = mediaTotal
    fullResultsCache.value = [...results.value]
  }

  /** 纯文本结果翻页（独立结果集，不惊动媒体结果）。 */
  async function loadTextEntryPage(page: number, text?: string) {
    const q = text ?? searchQuery.value
    if (!q) return
    textEntryPage.value = page
    try {
      const res = await invoke<PagedResponse<TextEntryHit>>('search_text_entries', {
        text: q,
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
    invalidateCache()
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
      currentPage.value = 1
      jumpPage.value = 1
      invalidateCache()
      await performSearch()
    }
  }

  function clearImageSearch() {
    if (!isImageSearch.value) return
    isImageSearch.value = false
    searchImagePath.value = ''
    // 第八轮修复：不清空 results —— 以图搜图取消后，若没有其它条件，
    // 用户应该回到「无结果」而不是「上一批以图搜图的结果仍挂在界面上」。
    if (isBrowsingMode.value) {
      results.value = []
      totalResults.value = 0
    }
  }

  async function loadBrowsePage(page: number) {
    try {
      const response: PagedResponse<SearchResult> = await invoke('list_all_files', {
        page,
        limit: pageSize.value
      })

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

  async function changePage(delta: number) {
    const newPage = currentPage.value + delta
    if (newPage < 1 || newPage > totalPages.value) return

    currentPage.value = newPage
    jumpPage.value = currentPage.value

    if (isBrowsingMode.value) {
      await loadBrowsePage(currentPage.value)
    } else {
      await performSearch()
    }
  }

  async function doJump(page?: number) {
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
    const currentId = ++searchRequestId
    isSearching.value = true
    try {
      searchQuery.value = ''
      isImageSearch.value = false
      searchImagePath.value = ''
      clearConditions()
      invalidateCache()

      const allItems: SearchResult[] = await invoke('get_all_files')
      if (currentId !== searchRequestId) return
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

  // 兜底清理：仅在整个应用卸载时全量清理。
  if (!lifecycleBound) {
    lifecycleBound = true
    onUnmounted(() => {
      clearAllVideoTimers()
      lifecycleBound = false
    })
  }

  return {
    // 状态
    searchQuery,
    conditions,
    groupOp,
    sortBy,
    quickTargets,
    allQuickTargetsOff,
    setQuickTarget,
    isImageSearch,
    searchImagePath,
    mainConditionLabel,
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
    estimateMsg,
    isEstimating,
    restorableLabel,
    // 条件行操作
    addCondition,
    removeCondition,
    updateCondition,
    clearConditions,
    setGroupOp,
    setSortBy,
    invalidateCache,
    conditionIndex,
    // 会话恢复
    restoreSession,
    dismissRestorable,
    hasRestorableSession: computed(() => restorableSession.value !== null),
    // 动作
    performSearch,
    estimateHits,
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