/**
 * useSelection — 多选状态管理（P1-10 / 第三轮「无多选 / 批量操作」）。
 *
 * 旧实现：App.vue 无 selection state，结果只存在于 UI 中。
 * 新实现：维护一个 Set<string>（按 path_key 去重），Shift 多选，
 * 候选集托盘展示选中项数，批量加笔记/加入智能文件夹/导出/清除。
 */
import { ref, computed } from 'vue'
import type { SearchResult } from '@/types/search'

// 模块级共享状态：多个组件（ResultCard / SelectionTray / App）共用同一份
const selectedPaths = ref<Set<string>>(new Set())
// path_key → 原始 path（批量加笔记/导出需要真实路径，而 selectedPaths 存的是 path_key）
const originalPaths = new Map<string, string>()
// P1-3：path_key → 完整 SearchResult（跨页导出需要完整数据，而非仅当前页）
const resultCache = new Map<string, SearchResult>()

function pathKey(path: string): string {
  return path.toLowerCase().replace(/\\/g, '/')
}

function remember(key: string, path: string): void {
  originalPaths.set(key, path)
}

export function useSelection() {
  const selectedCount = computed(() => selectedPaths.value.size)
  const hasSelection = computed(() => selectedPaths.value.size > 0)

  function isSelected(path: string): boolean {
    return selectedPaths.value.has(pathKey(path))
  }

  function toggle(path: string): void {
    const key = pathKey(path)
    if (selectedPaths.value.has(key)) {
      selectedPaths.value.delete(key)
      originalPaths.delete(key)
    } else {
      selectedPaths.value.add(key)
      remember(key, path)
    }
    // 触发响应式更新
    selectedPaths.value = new Set(selectedPaths.value)
  }

  function selectOne(path: string): void {
    selectedPaths.value.add(pathKey(path))
    remember(pathKey(path), path)
    selectedPaths.value = new Set(selectedPaths.value)
  }

  function deselectOne(path: string): void {
    const key = pathKey(path)
    selectedPaths.value.delete(key)
    originalPaths.delete(key)
    selectedPaths.value = new Set(selectedPaths.value)
  }

  function selectMany(paths: string[]): void {
    for (const p of paths) {
      selectedPaths.value.add(pathKey(p))
      remember(pathKey(p), p)
    }
    selectedPaths.value = new Set(selectedPaths.value)
  }

  function clear(): void {
    selectedPaths.value = new Set()
    originalPaths.clear()
  }

  function asArray(): string[] {
    return Array.from(selectedPaths.value)
  }

  /** 返回选中项的原始路径（而非 path_key），供批量加笔记等需要真实路径的操作 */
  function asOriginalArray(): string[] {
    return Array.from(selectedPaths.value).map((key) => originalPaths.get(key) ?? key)
  }

  /** P1-3：记录选中项的完整 SearchResult（供跨页导出取完整数据） */
  function rememberResult(item: SearchResult): void {
    resultCache.set(pathKey(item.path), item)
  }

  /** P1-3：返回所有选中项的完整 SearchResult（跨页，而非只滤当前页） */
  function getSelectedResults(): SearchResult[] {
    return Array.from(selectedPaths.value)
      .map((key) => resultCache.get(key))
      .filter((r): r is SearchResult => r !== undefined)
  }

  return {
    selectedPaths,
    selectedCount,
    hasSelection,
    isSelected,
    toggle,
    selectOne,
    deselectOne,
    selectMany,
    clear,
    asArray,
    asOriginalArray,
    rememberResult,
    getSelectedResults,
  }
}
