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
import type { SmartFolder, PagedResponse, SearchResult } from '@/types/search'
import { useSearch, normalizeResults } from './useSearch'
import { useClustering } from './useClustering'
import { t } from '@/i18n'

const smartFolders = ref<SmartFolder[]>([])

export function useSmartFolders() {
  const {
    searchQuery,
    s_vector,
    s_ocr,
    s_note,
    s_filename,
    isImageSearch,
    isShowAllMode,
    results,
    currentPage,
    jumpPage,
    pageSize,
    totalResults,
    searchMsg
  } = useSearch()
  const { isClusteringView } = useClustering()

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
      await invoke('save_smart_folder', {
        name: trimmed,
        queryText: searchQuery.value,
        useVector: s_vector.value,
        useOcr: s_ocr.value,
        useNote: s_note.value,
        useFilename: s_filename.value
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
    useVector: boolean, useOcr: boolean, useNote: boolean, useFilename: boolean) {
    try {
      await invoke('update_smart_folder', {
        id, name, queryText,
        useVector, useOcr, useNote, useFilename
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
   */
  async function refreshFolderCount(id: number): Promise<number | null> {
    try {
      const count = await invoke<number>('refresh_smart_folder_count', { id })
      // 就地更新 smartFolders 里对应项的 match_count
      const sf = smartFolders.value.find((s) => s.id === id)
      if (sf) sf.match_count = count
      return count
    } catch (e) {
      console.error('Failed to refresh folder count:', e)
      return null
    }
  }

  async function applySmartFolder(sf: SmartFolder) {
    // 把文件夹定义同步到 UI 过滤开关
    searchQuery.value = sf.query_text
    s_vector.value = sf.use_vector
    s_ocr.value = sf.use_ocr
    s_note.value = sf.use_note
    s_filename.value = sf.use_filename

    isImageSearch.value = false
    isClusteringView.value = false
    isShowAllMode.value = false
    currentPage.value = 1
    jumpPage.value = 1
    searchMsg.value = ''

    try {
      const response: PagedResponse<SearchResult> = await invoke('execute_smart_folder', {
        id: sf.id,
        page: currentPage.value,
        limit: pageSize.value
      })

      // 走统一的 normalizeResults：补齐 UI 字段、视频武装 25s 超时、低分图默认折叠
      results.value = normalizeResults(response.items, false)
      totalResults.value = response.total_count
    } catch (err) {
      searchMsg.value = t('smartFolder.execFailed', { err })
    }
  }

  return {
    smartFolders,
    loadSmartFolders,
    saveSmartFolder,
    updateSmartFolder,
    removeSmartFolder,
    refreshFolderCount,
    applySmartFolder
  }
}
