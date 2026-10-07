<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { TextEntryItem } from '@/types/search'
import PaginationBar from './PaginationBar.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import { useToast } from '@/composables/useToast'

const props = defineProps<{
  open: boolean
  entries: TextEntryItem[]
  total: number
  page: number
  totalPages: number
  pageSize: number
  query: string
  busy: boolean
  message: string
  selected: string[]
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'toggle', id: string): void
  (e: 'select-all-page'): void
  (e: 'update:page-size', size: number): void
  (e: 'delete'): void
  (e: 'pick-files'): void
  (e: 'pick-folder'): void
  (e: 'update:query', value: string): void
  (e: 'search'): void
  (e: 'change-page', delta: number): void
  (e: 'jump', page: number): void
}>()

// 全选本页判定：用于按钮文案「全选 / 取消全选」
const allPageSelected = computed(() =>
  props.entries.length > 0 && props.entries.every((e) => props.selected.includes(e.entry_id))
)

function fmtTime(ts: number): string {
  if (!ts) return '—'
  return new Date(ts * 1000).toLocaleString()
}

// P1-6：条目内容 3 行硬截断且无法展开（而这里正是「决定要不要删除」的地方）；
// 现加展开/折叠 + 复制。
const { push } = useToast()
const { t } = useI18n()
const expandedIds = ref<Set<string>>(new Set())
function toggleExpand(id: string) {
  const next = new Set(expandedIds.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  expandedIds.value = next
}
async function copyEntry(item: TextEntryItem) {
  try {
    await navigator.clipboard.writeText(item.content)
    push(t('cards.copied'), 'success')
  } catch {
    push(t('cards.copyFailed'), 'error')
  }
}

// P0-4：终态删除的二次确认（删除是终态不可回退，此前直接 emit 无确认）
const confirmOpen = ref(false)
function onRequestDelete() {
  if (props.selected.length === 0 || props.busy) return
  confirmOpen.value = true
}
function onConfirmDelete() {
  confirmOpen.value = false
  emit('delete')
}
</script>

<template>
  <div v-if="props.open" class="dialog-overlay" @click.self="emit('close')">
    <div class="dialog-panel">
      <h3 class="dialog-title">{{ $t('textManager.title') }}</h3>
      <p class="dialog-hint">
        {{ $t('textManager.hint', { total: props.total }) }}
      </p>

      <!-- 入库入口：系统只提供入口，导入哪些由用户选 -->
      <div class="ingest-row">
        <button class="btn btn-secondary" :disabled="props.busy" @click="emit('pick-files')">
          {{ $t('textManager.selectFiles') }}
        </button>
        <button class="btn btn-secondary" :disabled="props.busy" @click="emit('pick-folder')">
          {{ $t('textManager.selectFolder') }}
        </button>
      </div>

      <!-- 关键词过滤：条目多了必须先定位，才谈得上删除 -->
      <div class="search-row">
        <input
          :value="props.query"
          @input="emit('update:query', ($event.target as HTMLInputElement).value)"
          @keyup.enter="emit('search')"
          type="text"
          :placeholder="$t('textManager.filterPlaceholder')"
          class="entry-search-input"
        />
        <button class="btn btn-primary" :disabled="props.busy" @click="emit('search')">{{ $t('textManager.search') }}</button>
      </div>

      <!-- 批量操作：全选本页 + 每页条数（此前只能一条条勾选、固定 20 条） -->
      <div class="selection-row">
        <button class="btn btn-secondary btn-sm" :disabled="props.entries.length === 0" @click="emit('select-all-page')">
          {{ allPageSelected ? $t('textManager.deselectPage') : $t('textManager.selectAllPage') }}
        </button>
        <span class="page-size-select">
          {{ $t('textManager.perPage') }}
          <select
            :value="props.pageSize"
            class="custom-select-sm"
            @change="emit('update:page-size', Number(($event.target as HTMLSelectElement).value))"
          >
            <option :value="10">10</option>
            <option :value="20">20</option>
            <option :value="50">50</option>
            <option :value="100">100</option>
          </select>
        </span>
      </div>

      <!-- 条目列表：只读预览 + 勾选（打开时默认零勾选） -->
      <div class="entry-list">
        <label
          v-for="item in props.entries"
          :key="item.entry_id"
          class="entry-item"
          :class="{ selected: props.selected.includes(item.entry_id) }"
        >
          <input
            type="checkbox"
            :checked="props.selected.includes(item.entry_id)"
            @change="emit('toggle', item.entry_id)"
          />
          <div class="entry-body">
            <p
              class="entry-content"
              :class="{ 'entry-content-expanded': expandedIds.has(item.entry_id) }"
              :title="$t('textManager.clickToExpand')"
              @click="toggleExpand(item.entry_id)"
            >{{ item.content }}</p>
            <div class="entry-actions">
              <button class="btn-text" @click.stop="copyEntry(item)">{{ $t('cards.copy') }}</button>
              <button v-if="item.content.length > 120" class="btn-text" @click.stop="toggleExpand(item.entry_id)">
                {{ expandedIds.has(item.entry_id) ? $t('cards.collapse') : $t('cards.expand') }}
              </button>
            </div>
            <p class="entry-meta">
              <span class="entry-source">{{ item.source_uri || $t('textManager.noSource') }}</span>
              <span class="entry-time">{{ fmtTime(item.index_time) }}</span>
            </p>
          </div>
        </label>
        <p v-if="props.entries.length === 0" class="empty-hint">
          {{ props.query ? $t('textManager.noMatch') : $t('textManager.empty') }}
        </p>
      </div>

      <!-- 与搜索图片一致的分页（该场景下隐藏「Show All」入口） -->
      <PaginationBar
        v-if="props.total > 0"
        :current-page="props.page"
        :total-pages="props.totalPages"
        :total-results="props.total"
        :is-show-all-mode="false"
        :show-all-button="false"
        dense
        @change-page="emit('change-page', $event)"
        @jump="emit('jump', $event)"
      />

      <div class="dialog-actions">
        <span class="selected-count">{{ $t('textManager.selected', { count: props.selected.length }) }}</span>
        <button class="btn btn-secondary" @click="emit('close')">{{ $t('common.close') }}</button>
        <button
          class="btn btn-danger"
          :disabled="props.selected.length === 0 || props.busy"
          @click="onRequestDelete"
        >
          {{ $t('textManager.deleteSelected') }}
        </button>
      </div>

      <p v-if="props.message" class="dialog-msg">{{ props.message }}</p>

      <!-- P0-4：终态删除二次确认 -->
      <ConfirmDialog
        :visible="confirmOpen"
        :title="$t('textManager.deleteConfirmTitle')"
        :message="$t('textManager.deleteConfirmMessage', { count: props.selected.length })"
        :detail="$t('textManager.deleteConfirmDetail')"
        :confirm-text="$t('textManager.deleteSelected')"
        danger
        @confirm="onConfirmDelete"
        @cancel="confirmOpen = false"
      />
    </div>
  </div>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}
.dialog-panel {
  background: #12121a;
  border: 1px solid #2a2a3a;
  border-radius: 12px;
  padding: 22px;
  width: 620px;
  max-width: calc(100vw - 40px);
  max-height: calc(100vh - 60px);
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.dialog-title {
  margin: 0;
  font-size: 17px;
  color: #fff;
}
.dialog-hint {
  margin: 0;
  font-size: 12px;
  color: #888;
  line-height: 1.5;
}

.ingest-row,
.search-row {
  display: flex;
  gap: 10px;
  align-items: center;
}

.selection-row {
  display: flex;
  gap: 12px;
  align-items: center;
  justify-content: space-between;
}
.page-size-select {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #888;
}
.entry-search-input {
  flex: 1;
  min-width: 0;
  padding: 8px 10px;
  background: #0c0c12;
  color: #fff;
  border: 1px solid #2a2a3a;
  border-radius: 6px;
  font-size: 12px;
  outline: none;
}
.entry-search-input:focus {
  border-color: #8333ff;
}

.entry-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow-y: auto;
  max-height: 320px;
  border: 1px solid #1e1e2a;
  border-radius: 8px;
  padding: 10px;
  background: #0c0c12;
}
.entry-item {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  padding: 8px 10px;
  border-radius: 6px;
  border: 1px solid transparent;
  cursor: pointer;
}
.entry-item:hover {
  background: rgba(131, 51, 255, 0.08);
}
.entry-item.selected {
  border-color: #8333ff;
  background: rgba(131, 51, 255, 0.12);
}
.entry-body {
  flex: 1;
  min-width: 0;
}
.entry-content {
  margin: 0 0 4px 0;
  font-size: 12px;
  color: #ddd;
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
  word-break: break-word;
  cursor: pointer;
}
.entry-content-expanded {
  display: block;
  -webkit-line-clamp: unset;
  overflow: visible;
}
.entry-actions {
  display: flex;
  gap: 10px;
  margin-bottom: 4px;
}
.entry-meta {
  margin: 0;
  display: flex;
  justify-content: space-between;
  gap: 10px;
  font-size: 11px;
  color: #777;
}
.entry-source {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}
.entry-time {
  flex-shrink: 0;
}
.empty-hint {
  margin: 0;
  padding: 18px 0;
  text-align: center;
  font-size: 12px;
  color: #666;
}

.dialog-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
}
.selected-count {
  flex: 1;
  font-size: 12px;
  color: #888;
}
.dialog-msg {
  margin: 0;
  font-size: 12px;
  color: #67e5e5;
  line-height: 1.5;
}
</style>
