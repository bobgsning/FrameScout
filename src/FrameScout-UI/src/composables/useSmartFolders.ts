/**
 * useSmartFolders — 智能文件夹的读取、保存、删除与应用。
 * 应用文件夹时会同步搜索过滤开关，并把查询交给 Rust 后端原生执行。
 *
 * P1-3 修复（第三轮）：
 *   - promptSaveSmartFolder 改用应用内对话框回调（取消原生 prompt）；
 *   - 所有 invoke 包 try/catch（不再抛未捕获异常）；
 *   - 删除加二次确认；
 *   - refreshCount 走 refresh_smart_folder_count 命令拿真实计数（纯语义文件夹不再恒 0）。
 */
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type {
  SmartFolder,
  ConditionRow,
  ConditionTarget,
  GroupOp
} from '@/types/search'
import { useSearch } from './useSearch'
import { useClustering } from './useClustering'
import { t } from '@/i18n'

const smartFolders = ref<SmartFolder[]>([])

// M0'''：正在刷新计数的文件夹 id 集合。由 refreshFolderCount 真实完成时移除，
// 不再用「3 秒假装完成」的 setTimeout 兜底（见 SmartFolderBar 的 onRefresh）。
const refreshingFolders = ref<Set<number>>(new Set())

export function useSmartFolders() {
  const {
    searchQuery,
    conditions,
    groupOp,
    quickTargets,
    isImageSearch,
    isShowAllMode,
    currentPage,
    jumpPage,
    searchMsg,
    invalidateCache,
    setGroupOp,
    performSearch
  } = useSearch()
  const { isClusteringView } = useClustering()

  /** 第八轮 M011：把当前条件行序列化成 `criteria_json`。
   *  这一步修掉了一个静默行为：旧实现保存时只取「第一个文字条件 + 有没有画面条件」，
   *  排除、阈值、时间/目录/批次/类型条件全部**悄悄丢掉** —— 用户摆好一排条件，
   *  存成文件夹后语义就变了。现在完整存下来。 */
  function currentCriteria(): string | null {
    const conds = conditions.value.map((c) => ({ ...c }))
    if (!conds.length) return null
    return JSON.stringify({ groupOp: groupOp.value, conditions: conds })
  }

  /** 反序列化 `criteria_json`；容错（旧格式是裸数组 / 字段缺失时不抛）。 */
  function parseCriteria(json: string): { groupOp: GroupOp; conditions: ConditionRow[] } | null {
    try {
      const o = JSON.parse(json) as { groupOp?: string; conditions?: unknown }
      const arr = Array.isArray(o) ? o : o.conditions
      if (!Array.isArray(arr) || arr.length === 0) return null
      return {
        groupOp: !Array.isArray(o) && o.groupOp === 'any' ? 'any' : 'all',
        conditions: (arr as Array<Partial<ConditionRow>>).map((c, i) => ({
          ...c,
          id: c.id || `c_sf_${i}`,
          type: c.type || 'text'
        })) as ConditionRow[]
      }
    } catch {
      return null
    }
  }

  /** 从当前条件行提取「旧 7 字段」的通道设置。
   *  这 7 个字段现在只作为**旧文件夹的兼容载体**存在（criteria_json 为 null 时才被使用）；
   *  有条件行时以条件行为准。 */
  function currentChannelFlags() {
    const textCond = conditions.value.find((c) => c.type === 'text')
    const visualCond = conditions.value.find((c) => c.type === 'visual')
    const targets = textCond?.targets?.length ? textCond.targets : (['filename', 'note', 'ocr'] as ConditionTarget[])
    return {
      queryText: searchQuery.value || textCond?.value || '',
      useVector: !!visualCond,
      useOcr: targets.includes('ocr'),
      useNote: targets.includes('note'),
      useFilename: targets.includes('filename'),
      criteriaJson: currentCriteria()
    }
  }

  async function loadSmartFolders() {
    try {
      smartFolders.value = await invoke<SmartFolder[]>('get_smart_folders')
    } catch (e) {
      console.error('Failed to load smart folders:', e)
      searchMsg.value = t('smartFolder.loadFailed', { err: e })
    }
  }

  /**
   * 保存智能文件夹（P1-3：取消原生 prompt）。
   * 父组件（SmartFolderBar）传入 name，本函数只负责落库。
   */
  async function saveSmartFolder(name: string) {
    const trimmed = name.trim()
    if (!trimmed) {
      searchMsg.value = t('smartFolder.nameRequired')
      return
    }
    if (trimmed.length > 50) {
      searchMsg.value = t('smartFolder.nameTooLong')
      return
    }
    try {
      const flags = currentChannelFlags()
      await invoke('save_smart_folder', {
        name: trimmed,
        queryText: flags.queryText,
        useVector: flags.useVector,
        useOcr: flags.useOcr,
        useNote: flags.useNote,
        useFilename: flags.useFilename,
        criteriaJson: flags.criteriaJson
      })
      await loadSmartFolders()
      searchMsg.value = t('smartFolder.saved', { name: trimmed })
    } catch (e) {
      searchMsg.value = t('smartFolder.saveFailed', { err: e })
    }
  }

  /**
   * 更新智能文件夹（重命名 / 改查询条件）。
   */
  async function updateSmartFolder(id: number, name: string, queryText: string,
    useVector: boolean, useOcr: boolean, useNote: boolean, useFilename: boolean,
    criteriaJson?: string | null) {
    try {
      await invoke('update_smart_folder', {
        id, name, queryText,
        useVector, useOcr, useNote, useFilename,
        criteriaJson: criteriaJson ?? null
      })
      await loadSmartFolders()
    } catch (e) {
      searchMsg.value = t('smartFolder.updateFailed', { err: e })
    }
  }

  /**
   * 删除智能文件夹（P0-4：确认改为应用内 ConfirmDialog，由 SmartFolderBar 弹出，
   * 不再用原生 window.confirm）。
   * @param id 文件夹 id
   * @param name 文件夹名（用于提示）
   * @returns true 表示已删除
   */
  async function removeSmartFolder(id: number, name?: string): Promise<boolean> {
    const label = name || `#${id}`
    try {
      await invoke('delete_smart_folder', { id })
      await loadSmartFolders()
      searchMsg.value = t('smartFolder.deleted', { name: label })
      return true
    } catch (e) {
      searchMsg.value = t('smartFolder.deleteFailed', { err: e })
      return false
    }
  }

  /**
   * 刷新单个智能文件夹的真实计数（P1-3）。
   * 纯语义文件夹在列表视图里只能显示 `?`，这里走完整检索拿真实数字。
   * M0'''：`refreshingFolders` 由本函数真实完成时移除（finally），不再假装完成。
   */
  async function refreshFolderCount(id: number): Promise<number | null> {
    refreshingFolders.value.add(id)
    try {
      // 第八轮：条件行文件夹的求值在前端引擎（交/差/并），后端给不出真值。
      // 这种文件夹显示「?」，不假装刷新出一个数字来。
      const sf = smartFolders.value.find((s) => s.id === id)
      if (sf?.needs_frontend_eval) {
        sf.match_count = 0
        return null
      }
      const count = await invoke<number>('refresh_smart_folder_count', { id })
      // 就地更新 smartFolders 里对应项的 match_count
      if (sf) sf.match_count = count
      return count
    } catch (e) {
      console.error('Failed to refresh folder count:', e)
      return null
    } finally {
      refreshingFolders.value.delete(id)
    }
  }

  async function applySmartFolder(sf: SmartFolder) {
    const parsed = sf.criteria_json ? parseCriteria(sf.criteria_json) : null

    isImageSearch.value = false
    isClusteringView.value = false
    isShowAllMode.value = false
    currentPage.value = 1
    jumpPage.value = 1
    searchMsg.value = ''

    if (parsed) {
      // 第八轮：条件行文件夹直接由前端条件引擎求值 ——
      // 集合的交/差/并本来就在前端做，智能文件夹与搜索栏共用同一个求值器，
      // 因此「排除 / 阈值 / 时间 / 目录 / 批次 / 类型」在文件夹里同样成立。
      // 注意要清空搜索框：否则会额外生成一个「快速关键词」隐式条件，与条件行重复。
      searchQuery.value = ''
      quickTargets.value = { visual: false, filename: false, note: false, ocr: false, text: false }
      conditions.value = parsed.conditions
      setGroupOp(parsed.groupOp)
      invalidateCache()
      await performSearch()
      return
    }

    // 旧文件夹：7 字段映射成「快速关键词 + 通道选择」——这与快速搜索是同一种
    // OR 融合语义（视觉命中 或 任一字面命中），与 search_unified 单遍扫描同口径。
    // 走前端条件引擎执行，与搜索栏、条件行文件夹共用同一套求值器。
    // 第九轮：execute_smart_folder / search_images 已退役，旧文件夹不再走后端旧检索路径。
    searchQuery.value = sf.query_text
    conditions.value = []
    quickTargets.value = {
      visual: !!sf.use_vector,
      filename: !!sf.use_filename,
      note: !!sf.use_note,
      ocr: !!sf.use_ocr,
      text: false
    }
    setGroupOp('all')
    invalidateCache()
    await performSearch()
  }

  return {
    smartFolders,
    refreshingFolders,
    loadSmartFolders,
    saveSmartFolder,
    updateSmartFolder,
    removeSmartFolder,
    refreshFolderCount,
    applySmartFolder
  }
}
