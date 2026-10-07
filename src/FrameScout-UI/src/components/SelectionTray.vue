<script setup lang="ts">
/**
 * SelectionTray — 候选集托盘（P1-10 / 第三轮「找到之后什么都做不了」）。
 *
 * 底部浮窗：显示选中项数，提供批量操作：
 *   - 导出 CSV / Markdown / JSON
 *   - 批量加笔记（暂留接口，后端 update_note 不支持批量时只逐条调）
 *   - 加入智能文件夹
 *   - 清除选择
 */
import { useI18n } from 'vue-i18n'
import { useSelection } from '@/composables/useSelection'
import { useToast } from '@/composables/useToast'
import { exportCSV, exportMarkdown, exportJSON } from '@/utils/exporters'
import type { SearchResult } from '@/types/search'

const props = defineProps<{
  /** 当前可见结果列表（供导出时取完整数据） */
  results: SearchResult[]
}>()

const emit = defineEmits<{
  'batch-add-note': [paths: string[]]
  'add-to-smart-folder': [paths: string[]]
}>()

const { selectedCount, hasSelection, selectedPaths, asOriginalArray, clear, getSelectedResults, selectMany } = useSelection()
const { push } = useToast()
const { t } = useI18n()

// P1-3：跨页导出——优先取 useSelection 缓存的完整结果；若缓存缺失则退回当前页
function getItemsToExport(): SearchResult[] {
  const items = getSelectedResults()
  if (items.length > 0) return items
  const selectedSet = new Set(Array.from(selectedPaths.value).map((p) => p.toLowerCase().replace(/\\/g, '/')))
  return props.results.filter((r) => selectedSet.has(r.path.toLowerCase().replace(/\\/g, '/')))
}

function onExportCSV() {
  const items = getItemsToExport()
  if (items.length === 0) {
    push(t('tray.noSelection'), 'warning')
    return
  }
  exportCSV(items)
  push(t('tray.exportedCsv', { count: items.length }), 'success')
}

function onExportMarkdown() {
  const items = getItemsToExport()
  if (items.length === 0) {
    push(t('tray.noSelection'), 'warning')
    return
  }
  exportMarkdown(items)
  push(t('tray.exportedMd', { count: items.length }), 'success')
}

function onExportJSON() {
  const items = getItemsToExport()
  if (items.length === 0) {
    push(t('tray.noSelection'), 'warning')
    return
  }
  exportJSON(items)
  push(t('tray.exportedJson', { count: items.length }), 'success')
}

function onClear() {
  // P2-1：清除候选集提供「撤销」——保存清除前的选中项，撤销时恢复
  const prev = asOriginalArray()
  clear()
  push(t('tray.cleared'), 'info', undefined, {
    label: t('common.undo'),
    onClick: () => selectMany(prev)
  })
}
</script>

<template>
  <Transition name="tray-slide">
    <div v-if="hasSelection" class="selection-tray">
      <div class="tray-info">
        <span class="tray-count-badge">{{ selectedCount }}</span>
        <span class="tray-label">{{ $t('tray.selected') }}</span>
      </div>
      <div class="tray-actions">
        <button class="tray-btn" @click="onExportCSV" :title="$t('tray.exportCsv')">📊 CSV</button>
        <button class="tray-btn" @click="onExportMarkdown" :title="$t('tray.exportMd')">📝 MD</button>
        <button class="tray-btn" @click="onExportJSON" :title="$t('tray.exportJson')">{ } JSON</button>
        <span class="tray-divider" />
        <button
          class="tray-btn"
          @click="emit('batch-add-note', asOriginalArray())"
          :title="$t('tray.addNoteHint')"
        >{{ $t('tray.addNote') }}</button>
        <button
          class="tray-btn"
          disabled
          :title="$t('tray.smartFolderHint')"
        >{{ $t('tray.addFolder') }}</button>
        <button class="tray-btn tray-clear" @click="onClear" :title="$t('tray.clearSelection')">✕</button>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.selection-tray {
  position: fixed;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 9000;
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 12px 20px;
  background: linear-gradient(135deg, rgba(28, 28, 46, 0.95), rgba(20, 20, 30, 0.95));
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(108, 142, 227, 0.3);
  border-radius: 16px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.4);
}

.tray-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.tray-count-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 28px;
  height: 28px;
  padding: 0 8px;
  background: linear-gradient(135deg, #6c8ee3, #4a6fcf);
  color: #fff;
  font-size: 14px;
  font-weight: 700;
  border-radius: 14px;
}

.tray-label {
  font-size: 13px;
  color: #b0b0c0;
}

.tray-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.tray-btn {
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  color: #f0f0f5;
  font-size: 12px;
  cursor: pointer;
  padding: 7px 12px;
  transition: all 0.15s;
  white-space: nowrap;
  font-family: 'Noto Sans', system-ui, sans-serif;
}

.tray-btn:hover {
  background: rgba(108, 142, 227, 0.18);
  border-color: rgba(108, 142, 227, 0.35);
}

.tray-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.tray-btn:disabled:hover {
  background: rgba(255, 255, 255, 0.06);
  border-color: rgba(255, 255, 255, 0.08);
}

.tray-clear {
  color: #f87171;
  border-color: rgba(248, 113, 113, 0.2);
}

.tray-clear:hover {
  background: rgba(248, 113, 113, 0.15);
  border-color: rgba(248, 113, 113, 0.35);
}

.tray-divider {
  width: 1px;
  height: 20px;
  background: rgba(255, 255, 255, 0.1);
  margin: 0 4px;
}

/* 托盘滑入动画 */
.tray-slide-enter-active,
.tray-slide-leave-active {
  transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.tray-slide-enter-from,
.tray-slide-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(80px);
}
</style>
