<script setup lang="ts">
/**
 * SmartFolderBar — 智能文件夹 pill 列表（P1-3 升级）。
 *
 * 第三轮修复：
 *   - pill 显示查询条件 tooltip（hover 看完整条件）；
 *   - 计数显示去黑话（不再 `(Num: 12 )`，改 `12` 徽标）；
 *   - `?` 加 tooltip 解释「需点击执行才知道」；
 *   - 删除按钮独立 hover 区域，不再与 pill 整体混在一起（减少误点）；
 *   - 删除走二次确认（由 useSmartFolders.removeSmartFolder 内部 confirm）；
 *   - 加刷新计数按钮（纯语义文件夹可手动拿真实数字）。
 */
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SmartFolder } from '@/types/search'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

defineProps<{
  smartFolders: SmartFolder[]
}>()

const emit = defineEmits<{
  (e: 'apply', folder: SmartFolder): void
  (e: 'remove', id: number, name?: string): void
  (e: 'refreshCount', id: number): void
  (e: 'edit', folder: SmartFolder): void
}>()

// 正在刷新计数的文件夹 id 集合
const { t } = useI18n()
const refreshing = ref<Set<number>>(new Set())

// P0-4：删除二次确认（应用内 ConfirmDialog，替代 window.confirm）
const confirmOpen = ref(false)
const pendingDelete = ref<SmartFolder | null>(null)
function onRequestDelete(sf: SmartFolder) {
  pendingDelete.value = sf
  confirmOpen.value = true
}
function onConfirmDelete() {
  if (pendingDelete.value) emit('remove', pendingDelete.value.id, pendingDelete.value.name)
  confirmOpen.value = false
  pendingDelete.value = null
}

function buildTooltip(sf: SmartFolder): string {
  const parts: string[] = [t('smartFolder.query', { query: sf.query_text || t('smartFolder.empty') })]
  const channels: string[] = []
  if (sf.use_vector) channels.push(t('smartFolder.visualSemantic'))
  if (sf.use_ocr) channels.push(t('smartFolder.ocr'))
  if (sf.use_note) channels.push(t('smartFolder.note'))
  if (sf.use_filename) channels.push(t('smartFolder.filename'))
  parts.push(t('smartFolder.channels', { channels: channels.join(' / ') || t('smartFolder.none') }))
  if (sf.match_count === 0 && sf.use_vector && !(sf.use_ocr || sf.use_note || sf.use_filename)) {
    parts.push(t('smartFolder.countHint'))
  }
  return parts.join('\n')
}

function onRefresh(e: MouseEvent, id: number) {
  e.stopPropagation()
  refreshing.value.add(id)
  emit('refreshCount', id)
  // 由父组件在刷新完成后移除 id；这里 3s 兜底自动移除
  setTimeout(() => refreshing.value.delete(id), 3000)
}
</script>

<template>
  <div v-if="smartFolders.length > 0" class="smart-folders-bar">
    <span class="smart-folder-label">{{ $t('smartFolder.title') }}</span>

    <div
      v-for="sf in smartFolders"
      :key="sf.id"
      class="smart-folder-pill"
      :title="buildTooltip(sf)"
      @click="emit('apply', sf)"
    >
      <span class="sf-name">📁 {{ sf.name }}</span>
      <span
        class="sf-count-badge"
        :class="{ 'sf-count-unknown': sf.match_count === 0 && sf.use_vector && !(sf.use_ocr || sf.use_note || sf.use_filename) }"
        :title="sf.match_count === 0 && sf.use_vector ? $t('smartFolder.semanticOnly') : $t('smartFolder.matchCount', { count: sf.match_count })"
      >
        {{ sf.match_count === 0 && sf.use_vector && !(sf.use_ocr || sf.use_note || sf.use_filename) ? '?' : sf.match_count }}
      </span>
      <button
        class="sf-refresh"
        :disabled="refreshing.has(sf.id)"
        @click="onRefresh($event, sf.id)"
        :title="refreshing.has(sf.id) ? $t('smartFolder.refreshing') : $t('smartFolder.refreshCount')"
      >↻</button>
      <button
        class="sf-edit"
        @click.stop="emit('edit', sf)"
        :title="$t('smartFolder.edit')"
      >✏️</button>
      <button
        class="sf-delete"
        @click.stop="onRequestDelete(sf)"
        :title="$t('smartFolder.delete')"
      >✕</button>
    </div>

    <!-- P0-4：智能文件夹删除二次确认（应用内，替代 window.confirm） -->
    <ConfirmDialog
      :visible="confirmOpen"
      :title="$t('smartFolder.deleteConfirmTitle')"
      :message="$t('smartFolder.confirmDelete', { name: pendingDelete?.name || '#' + (pendingDelete?.id ?? '') })"
      :confirm-text="$t('smartFolder.delete')"
      danger
      @confirm="onConfirmDelete"
      @cancel="confirmOpen = false"
    />
  </div>
</template>

<style scoped>
.smart-folders-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  margin-bottom: 20px;
  flex-wrap: wrap;
}

.smart-folder-label {
  font-size: 12px;
  color: #8888a0;
  font-weight: bold;
}

.smart-folder-pill {
  display: flex;
  align-items: center;
  gap: 6px;
  background: linear-gradient(135deg, rgba(20, 20, 31, 0.95), rgba(28, 28, 46, 0.95));
  border: 1px solid rgba(108, 142, 227, 0.2);
  padding: 5px 8px 5px 12px;
  border-radius: 20px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.18s;
  white-space: nowrap;
}
.smart-folder-pill:hover {
  border-color: rgba(108, 142, 227, 0.5);
  background: linear-gradient(135deg, rgba(28, 28, 46, 0.95), rgba(40, 40, 60, 0.95));
  box-shadow: 0 2px 12px rgba(108, 142, 227, 0.12);
}

.sf-name {
  color: #f0f0f5;
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sf-count-badge {
  color: #6c8ee3;
  font-size: 11px;
  font-weight: 600;
  min-width: 16px;
  text-align: center;
}
.sf-count-badge.sf-count-unknown {
  color: #8888a0;
}

.sf-refresh,
.sf-edit,
.sf-delete {
  background: none;
  border: none;
  font-size: 11px;
  cursor: pointer;
  padding: 0 4px;
  border-radius: 50%;
  transition: all 0.15s;
  line-height: 1;
}

.sf-edit {
  color: #666677;
}
.sf-edit:hover {
  color: #6c8ee3;
  background: rgba(108, 142, 227, 0.15);
}

.sf-refresh {
  color: #666677;
}
.sf-refresh:hover:not(:disabled) {
  color: #6c8ee3;
  background: rgba(108, 142, 227, 0.15);
}
.sf-refresh:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.sf-delete {
  color: #666677;
}
.sf-delete:hover {
  color: #f87171;
  background: rgba(248, 113, 113, 0.15);
}
</style>
