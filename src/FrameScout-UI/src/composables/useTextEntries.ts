/**
 * useTextEntries — 纯文本通道的资产管理：条目列表、删除、文本文件/文件夹入库。
 *
 * 与 useSearch 的分工：`useSearch` 管「检索」（把 query 变成命中），
 * 本 composable 管「条目资产」（库里有哪些条目、怎么增删）。
 *
 * 设计原则（与幽灵清理一致）：**先预览、后行动**。
 * 打开管理面板即列出条目（只读），删除只作用于用户显式勾选的 entry_id 列表——
 * 删除是终态不可回退，绝不提供「一键清空」。
 */
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { TextEntryItem, TextEntryListResponse, TextIngestResult } from '@/types/search'
import { t } from '@/i18n'

// ---------- 模块级共享状态（单例） ----------
const managerOpen = ref(false)
const textEntries = ref<TextEntryItem[]>([])
const textEntriesTotal = ref(0)
const textEntriesPage = ref(1)
const textEntriesPageSize = ref(20)
/** 管理面板的关键词过滤（按条目内容子串匹配，便于定位到想删的那几条） */
const textEntriesQuery = ref('')
const textEntriesTotalPages = computed(() =>
  Math.max(1, Math.ceil(textEntriesTotal.value / textEntriesPageSize.value))
)
/** 用户勾选待删除的 entry_id（打开面板时为空，系统不预设任何选择） */
const selectedEntryIds = ref<string[]>([])
const textEntriesBusy = ref(false)
const textEntriesMsg = ref('')

export function useTextEntries() {
  async function loadTextEntries(page: number = textEntriesPage.value) {
    textEntriesBusy.value = true
    try {
      const res: TextEntryListResponse = await invoke('list_text_entries', {
        page,
        limit: textEntriesPageSize.value,
        query: textEntriesQuery.value
      })
      textEntries.value = res.items
      textEntriesTotal.value = res.total_count
      textEntriesPage.value = page
    } catch (err) {
      textEntriesMsg.value = t('textEntries.loadFailed', { err })
    } finally {
      textEntriesBusy.value = false
    }
  }

  /** 关键词过滤：从第 1 页重新加载 */
  async function searchTextEntries() {
    await loadTextEntries(1)
  }

  async function changeTextEntriesPage(delta: number) {
    const next = textEntriesPage.value + delta
    if (next < 1 || next > textEntriesTotalPages.value) return
    await loadTextEntries(next)
  }

  async function jumpTextEntriesPage(page: number) {
    let target = page
    if (target < 1) target = 1
    if (target > textEntriesTotalPages.value) target = textEntriesTotalPages.value
    await loadTextEntries(target)
  }

  function openManager() {
    managerOpen.value = true
    selectedEntryIds.value = []
    textEntriesQuery.value = ''
    textEntriesMsg.value = ''
    loadTextEntries(1)
  }

  function closeManager() {
    managerOpen.value = false
  }

  function toggleEntry(id: string) {
    const i = selectedEntryIds.value.indexOf(id)
    if (i >= 0) {
      selectedEntryIds.value.splice(i, 1)
    } else {
      selectedEntryIds.value.push(id)
    }
  }

  // 全选/反选本页（此前只能一条条勾选）
  function toggleSelectAllPage() {
    const pageIds = textEntries.value.map((e) => e.entry_id)
    const allSelected = pageIds.length > 0 && pageIds.every((id) => selectedEntryIds.value.includes(id))
    if (allSelected) {
      selectedEntryIds.value = selectedEntryIds.value.filter((id) => !pageIds.includes(id))
    } else {
      for (const id of pageIds) {
        if (!selectedEntryIds.value.includes(id)) selectedEntryIds.value.push(id)
      }
    }
  }

  // 每页条数（与主页面一致的选项）
  async function changeTextEntriesPageSize(size: number) {
    textEntriesPageSize.value = size
    selectedEntryIds.value = []
    await loadTextEntries(1)
  }

  async function deleteSelected() {
    if (selectedEntryIds.value.length === 0) return
    textEntriesBusy.value = true
    try {
      const removed: number = await invoke('delete_text_entries', {
        entryIds: selectedEntryIds.value
      })
      textEntriesMsg.value = t('textEntries.deleted', { count: removed })
      selectedEntryIds.value = []
      await loadTextEntries(1)
    } catch (err) {
      textEntriesMsg.value = t('textEntries.deleteFailed', { err })
    } finally {
      textEntriesBusy.value = false
    }
  }

  /** 选择文本文件（多选）后入库 */
  async function pickTextFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: 'Text', extensions: ['md', 'markdown', 'txt', 'text'] }]
    })
    if (!selected || (Array.isArray(selected) && selected.length === 0)) return
    const paths = Array.isArray(selected) ? (selected as string[]) : [selected as string]
    await ingestPaths(paths)
  }

  /** 选择文件夹后递归入库其中的文本文件 */
  async function pickTextFolder() {
    const selected = await open({ directory: true })
    if (!selected) return
    await ingestPaths([selected as string])
  }

  async function ingestPaths(paths: string[]) {
    textEntriesBusy.value = true
    textEntriesMsg.value = t('textEntries.ingesting')
    try {
      const res: TextIngestResult = await invoke('ingest_text_files', { paths })
      const failedNote = res.failed.length > 0 ? ' · ' + t('textEntries.failedCount', { count: res.failed.length }) : ''
      textEntriesMsg.value = t('textEntries.ingested', { files: res.ingested_files.length, chunks: res.chunks, failedNote })
      await loadTextEntries(1)
    } catch (err) {
      textEntriesMsg.value = t('textEntries.ingestFailed', { err })
    } finally {
      textEntriesBusy.value = false
    }
  }

  return {
    // state
    managerOpen,
    textEntries,
    textEntriesTotal,
    textEntriesPage,
    textEntriesPageSize,
    textEntriesTotalPages,
    textEntriesQuery,
    selectedEntryIds,
    textEntriesBusy,
    textEntriesMsg,
    // actions
    loadTextEntries,
    searchTextEntries,
    changeTextEntriesPage,
    jumpTextEntriesPage,
    openManager,
    closeManager,
    toggleEntry,
    toggleSelectAllPage,
    changeTextEntriesPageSize,
    deleteSelected,
    pickTextFiles,
    pickTextFolder
  }
}
