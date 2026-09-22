/**
 * useSmartFolders — 智能文件夹的读取、保存、删除与应用。
 * 应用文件夹时会同步搜索过滤开关，并把查询交给 Rust 后端原生执行。
 */
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { SmartFolder, PagedResponse, SearchResult } from '@/types/search'
import { useSearch, normalizeResults } from './useSearch'
import { useClustering } from './useClustering'

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
      console.error(e)
    }
  }

  async function promptSaveSmartFolder() {
    const name = prompt('Enter a name for this Smart Folder:', searchQuery.value)
    if (!name) return

    await invoke('save_smart_folder', {
      name,
      queryText: searchQuery.value,
      useVector: s_vector.value,
      useOcr: s_ocr.value,
      useNote: s_note.value,
      useFilename: s_filename.value
    })
    await loadSmartFolders()
  }

  async function removeSmartFolder(id: number) {
    await invoke('delete_smart_folder', { id })
    await loadSmartFolders()
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
      searchMsg.value = `⚠️ Smart Folder execution failed: ${err}`
    }
  }

  return {
    smartFolders,
    loadSmartFolders,
    promptSaveSmartFolder,
    removeSmartFolder,
    applySmartFolder
  }
}
