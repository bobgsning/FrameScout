<script setup lang="ts">
/**
 * SearchConsole — 搜索控制台（第七轮重构：快速关键词 + 多条件行）。
 * 直接消费 useSearch 单例（条件状态不再是 props 透传），搜索框只做纯关键词，
 * 逻辑交给下面的条件行。
 */
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { getAssetUrl } from '@/utils/media'
import { useSearch } from '@/composables/useSearch'
import { usePreferences } from '@/composables/usePreferences'
import ConditionRow from './ConditionRow.vue'
import type { SortBy } from '@/composables/useSearch'
import type { ConditionType, GroupOp, BatchReport } from '@/types/search'

const {
  searchQuery,
  conditions,
  groupOp,
  sortBy,
  quickTargets,
  allQuickTargetsOff,
  setQuickTarget,
  isImageSearch,
  searchImagePath,
  searchMsg,
  mainConditionLabel,
  estimateMsg,
  isEstimating,
  restorableLabel,
  restoreSession,
  dismissRestorable,
  resetAndSearch,
  searchByImageAction,
  clearImageSearch,
  addCondition,
  removeCondition,
  updateCondition,
  clearConditions,
  setGroupOp,
  setSortBy,
  estimateHits
} = useSearch()

const { t } = useI18n()

const emit = defineEmits<{
  (e: 'save-smart-folder'): void
}>()

// ---------- 搜索历史 ----------
const { searchHistory, removeSearchHistoryEntry, clearSearchHistory } = usePreferences()
const historyOpen = ref(false)
function onSearchFocus() {
  if (searchHistory.value.length > 0) historyOpen.value = true
}
function onSearchBlur() {
  setTimeout(() => {
    historyOpen.value = false
  }, 150)
}
function pickHistory(q: string) {
  searchQuery.value = q
  historyOpen.value = false
  resetAndSearch()
}
function removeHistory(q: string) {
  removeSearchHistoryEntry(q)
}
function clearHistory() {
  clearSearchHistory()
  historyOpen.value = false
}

// ---------- 添加条件 ----------
const addMenuOpen = ref(false)
// 第八轮：file_type 与 seed 此前只有后端实现、UI 不可达（死类型），现已补进菜单。
const addTypeOptions: { value: ConditionType; labelKey: string }[] = [
  { value: 'text', labelKey: 'cond.typeText' },
  { value: 'visual', labelKey: 'cond.typeVisual' },
  { value: 'exclude', labelKey: 'cond.typeExclude' },
  { value: 'time', labelKey: 'cond.typeTime' },
  { value: 'path', labelKey: 'cond.typePath' },
  { value: 'batch', labelKey: 'cond.typeBatch' },
  { value: 'file_type', labelKey: 'cond.typeFileType' },
  { value: 'seed', labelKey: 'cond.typeSeed' },
  { value: 'text_semantic', labelKey: 'cond.typeTextSemantic' },
  { value: 'text_entry', labelKey: 'cond.typeTextEntry' }
]
function onAddCondition(type: ConditionType) {
  addCondition(type)
  addMenuOpen.value = false
  resetAndSearch()
}

// ---------- 来源批次（人类可读标签，只取扫描批次） ----------
const scanBatches = ref<BatchReport[]>([])
async function loadScanBatches() {
  try {
    scanBatches.value = await invoke<BatchReport[]>('list_scan_reports', {
      limit: 100,
      batchType: 'scan'
    })
  } catch {
    scanBatches.value = []
  }
}
onMounted(loadScanBatches)

// ---------- 生效条件数与排序 ----------
const effectiveCount = computed(() => conditions.value.length + (searchQuery.value.trim() ? 1 : 0))
const mainLabel = computed(() => mainConditionLabel.value || t('cond.defaultSort'))

function onSearch() {
  resetAndSearch()
}
function onGroupOpChange(e: Event) {
  setGroupOp((e.target as HTMLSelectElement).value as GroupOp)
  resetAndSearch()
}
// 第八轮：排序选择器（此前只有一行静态文本，用户改不了）。
// 选「入库时间 / 文件名」时走 L3 全量 + 前端排序，保证跨页全局有序。
function onSortByChange(e: Event) {
  setSortBy((e.target as HTMLSelectElement).value as SortBy)
  resetAndSearch()
}

function onRestore() {
  restoreSession()
  resetAndSearch()
}

// 快速搜索通道：勾选/取消，处理「文本语义」与其它四个互斥
type QuickChannelKey = 'visual' | 'filename' | 'note' | 'ocr' | 'text'
function onQuickTargetChange(key: QuickChannelKey, e: Event) {
  const checked = (e.target as HTMLInputElement).checked
  setQuickTarget(key, checked)
  resetAndSearch()
}
</script>

<template>
  <div class="search-console">
    <!-- 第八轮：上次搜过什么 —— 只提示、不自动搜索（行动由用户触发）。 -->
    <div v-if="restorableLabel" class="session-hint">
      <span>{{ $t('searcher.lastSession', { what: restorableLabel }) }}</span>
      <button class="btn btn-primary btn-sm" @click="onRestore">{{ $t('searcher.restoreSession') }}</button>
      <button class="btn-ghost" :title="$t('searcher.dismissSession')" @click="dismissRestorable">✕</button>
    </div>

    <div class="search-bar">
      <div class="search-input-wrap">
        <input
          v-model="searchQuery"
          @keyup.enter="onSearch"
          @focus="onSearchFocus"
          @blur="onSearchBlur"
          id="searchQuery"
          type="text"
          :placeholder="$t('search.placeholder')"
          class="custom-input search-input"
        />
        <button
          v-if="searchQuery"
          class="clear-query"
          type="button"
          :title="$t('search.clearQuery')"
          @mousedown.prevent="searchQuery = ''"
        >✕</button>

        <div v-if="historyOpen && searchHistory.length > 0" class="history-dropdown">
          <div class="history-header">
            <span>{{ $t('search.history') }}</span>
            <button class="history-clear" type="button" @mousedown.prevent="clearHistory">
              {{ $t('search.clearHistory') }}
            </button>
          </div>
          <ul class="history-list">
            <li v-for="q in searchHistory" :key="q" class="history-item">
              <button class="history-term" type="button" @mousedown.prevent="pickHistory(q)">{{ q }}</button>
              <button class="history-remove" type="button" :title="$t('search.removeHistory')" @mousedown.prevent="removeHistory(q)">✕</button>
            </li>
          </ul>
        </div>
      </div>
      <button class="btn btn-primary search-btn" @click="onSearch">{{ $t('search.search') }}</button>
      <button class="btn btn-outline-primary" :title="$t('search.similarHint')" @click="searchByImageAction">
        {{ $t('search.similar') }}
      </button>
      <button
        class="btn btn-secondary"
        :title="$t('search.saveSmartFolderHint')"
        :disabled="!searchQuery && conditions.length === 0"
        @click="emit('save-smart-folder')"
      >
        {{ $t('search.save') }}
      </button>
    </div>

    <!-- 快速搜索的通道选择（画面语义 / 文件名 / 笔记 / OCR / 文本语义）——主搜索框保留「搜什么」的能力；
         「文本语义」（BGE）与其它四个互斥 -->
    <div class="quick-targets">
      <span class="qt-label">{{ $t('search.target') }}</span>
      <label class="qt-item" :class="{ off: !quickTargets.visual }">
        <input type="checkbox" :checked="quickTargets.visual" @change="onQuickTargetChange('visual', $event)" />
        {{ $t('search.visual') }}
      </label>
      <label class="qt-item" :class="{ off: !quickTargets.filename }">
        <input type="checkbox" :checked="quickTargets.filename" @change="onQuickTargetChange('filename', $event)" />
        {{ $t('search.filename') }}
      </label>
      <label class="qt-item" :class="{ off: !quickTargets.note }">
        <input type="checkbox" :checked="quickTargets.note" @change="onQuickTargetChange('note', $event)" />
        {{ $t('search.note') }}
      </label>
      <label class="qt-item" :class="{ off: !quickTargets.ocr }">
        <input type="checkbox" :checked="quickTargets.ocr" @change="onQuickTargetChange('ocr', $event)" />
        {{ $t('search.ocr') }}
      </label>
      <span class="qt-sep"></span>
      <label class="qt-item qt-item-bge" :class="{ off: !quickTargets.text }">
        <input type="checkbox" :checked="quickTargets.text" @change="onQuickTargetChange('text', $event)" />
        {{ $t('search.textSemantic') }}
      </label>
    </div>

    <!-- 以图搜图目标图横幅（搜索词不参与，仅视觉） -->
    <div v-if="isImageSearch && searchImagePath" class="visual-target-banner">
      <img :src="getAssetUrl(searchImagePath)" class="target-img" />
      <div class="target-info">
        <p class="target-title">{{ $t('search.visualTarget') }}</p>
        <p class="target-path">{{ searchImagePath }}</p>
        <p class="target-note">{{ $t('search.visualIgnoresQuery') }}</p>
      </div>
      <button class="btn btn-danger btn-sm" @click="clearImageSearch">{{ $t('search.cancel') }}</button>
    </div>

    <!-- 条件面板 -->
    <div class="cond-panel">
      <div class="cond-panel-head">
        <span class="cond-group-label">{{ $t('cond.groupLabel') }}</span>
        <select class="cond-group-select" :value="groupOp" @change="onGroupOpChange">
          <option value="all">{{ $t('cond.groupAll') }}</option>
          <option value="any">{{ $t('cond.groupAny') }}</option>
        </select>
        <span class="cond-count">{{ $t('cond.activeCount', { n: effectiveCount }) }}</span>
      </div>

      <div v-if="conditions.length" class="cond-list">
        <ConditionRow
          v-for="(c, i) in conditions"
          :key="c.id"
          :condition="c"
          :index="i + 1"
          :batches="scanBatches"
          @update="(patch) => updateCondition(c.id, patch)"
          @remove="removeCondition(c.id)"
        />
      </div>
      <p v-else class="cond-empty">{{ $t('cond.empty') }}</p>

      <div class="cond-panel-foot">
        <div class="add-cond-wrap">
          <button class="btn-add-cond" @click="addMenuOpen = !addMenuOpen">＋ {{ $t('cond.addCondition') }}</button>
          <div v-if="addMenuOpen" class="add-cond-menu">
            <button
              v-for="opt in addTypeOptions"
              :key="opt.value"
              class="add-cond-item"
              @click="onAddCondition(opt.value)"
            >{{ $t(opt.labelKey) }}</button>
          </div>
        </div>
        <button v-if="conditions.length" class="btn-clear-cond" @click="clearConditions(); resetAndSearch()">
          {{ $t('cond.clearAll') }}
        </button>
      </div>

      <!-- 排序（主条件排序：分数有明确定义域，不编综合分） -->
      <div class="sort-row">
        <span class="sort-label">{{ $t('cond.sortBy') }}</span>
        <select class="cond-group-select" :value="sortBy" @change="onSortByChange">
          <option value="main">{{ $t('cond.sortByMain', { label: mainLabel }) }}</option>
          <option value="index_time">{{ $t('cond.sortByTime') }}</option>
          <option value="filename">{{ $t('cond.sortByName') }}</option>
        </select>
        <button class="btn-estimate" :disabled="isEstimating" :title="$t('cond.estimateHint')" @click="estimateHits">
          {{ isEstimating ? $t('cond.estimating') : $t('cond.estimate') }}
        </button>
        <span v-if="estimateMsg" class="estimate-msg">{{ estimateMsg }}</span>
      </div>
    </div>

    <p v-if="searchMsg" class="error-msg">{{ searchMsg }}</p>

    <!-- 第九轮：所有搜索通道都被取消 —— 全取消会回退到默认字面三通道，如实告知（暂不可关闭，重新勾选任一通道即消失）。 -->
    <div v-if="allQuickTargetsOff" class="all-targets-off-hint" role="status">
      {{ $t('search.allTargetsOffHint') }}
    </div>
  </div>
</template>

<style scoped>
.search-console {
  background: #12121a;
  padding: 25px;
  border-radius: 12px;
  max-width: 800px;
  margin: 0 auto 30px auto;
  border: 1px solid #222233;
}

.search-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-wrap: wrap;
  gap: 10px;
}

.search-input-wrap {
  position: relative;
  flex: 1 1 auto;
  max-width: 480px;
  min-width: 0;
}

.search-input {
  width: 100%;
  max-width: 450px;
  font-size: 16px;
  border-color: #8333ff;
}

.search-bar > button {
  flex-shrink: 0;
}

/* 快速搜索的通道选择（画面语义 / 文件名 / 笔记 / OCR） */
.quick-targets {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-wrap: wrap;
  gap: 14px;
  margin-top: 10px;
}
.qt-label {
  font-size: 12px;
  color: #8888a0;
}
.qt-item {
  font-size: 13px;
  color: #c8c8d8;
  cursor: pointer;
  white-space: nowrap;
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.qt-item input {
  accent-color: #6c8ee3;
  cursor: pointer;
}
.qt-item.off {
  color: #666677;
}
.qt-sep {
  width: 1px;
  height: 14px;
  background: rgba(255, 255, 255, 0.1);
}
.qt-item-bge {
  color: #b06cff;
}

/* 第九轮：所有搜索通道被取消时的固定提示（右下角，暂不可关闭） */
.all-targets-off-hint {
  position: fixed;
  right: 20px;
  bottom: 20px;
  z-index: 900;
  max-width: 340px;
  padding: 12px 16px;
  background: #1f1a12;
  border: 1px solid #d19a3c;
  border-radius: 10px;
  color: #ffd88a;
  font-size: 13px;
  line-height: 1.5;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
}

.clear-query {
  position: absolute;
  right: 10px;
  top: 50%;
  transform: translateY(-50%);
  background: rgba(255, 255, 255, 0.06);
  border: none;
  color: #8888a0;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  font-size: 12px;
  line-height: 1;
  cursor: pointer;
  transition: color 0.15s, background 0.15s;
}
.clear-query:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.14);
}

/* 第八轮：上次搜索提示条 */
.session-hint {
  display: flex;
  align-items: center;
  gap: 8px;
  justify-content: center;
  flex-wrap: wrap;
  margin-bottom: 12px;
  padding: 8px 12px;
  font-size: 12px;
  color: #b0b0c0;
  background: rgba(108, 142, 227, 0.1);
  border: 1px solid rgba(108, 142, 227, 0.28);
  border-radius: 8px;
}
.session-hint .btn-ghost {
  background: none;
  border: none;
  color: #666677;
  cursor: pointer;
  font-size: 12px;
}
.session-hint .btn-ghost:hover {
  color: #f0f0f5;
}

/* 第八轮：试算按钮与结果 */
.btn-estimate {
  padding: 4px 12px;
  font-size: 12px;
  border-radius: 6px;
  border: 1px solid rgba(108, 142, 227, 0.4);
  background: rgba(108, 142, 227, 0.12);
  color: #cdd8f7;
  cursor: pointer;
}
.btn-estimate:hover:not(:disabled) {
  background: rgba(108, 142, 227, 0.24);
}
.btn-estimate:disabled {
  opacity: 0.5;
  cursor: default;
}
.estimate-msg {
  font-size: 12px;
  color: #9fb0d8;
}

.history-dropdown {
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  right: 0;
  background: #16161f;
  border: 1px solid #2a2a3a;
  border-radius: 8px;
  box-shadow: 0 12px 30px rgba(0, 0, 0, 0.5);
  z-index: 50;
  overflow: hidden;
}
.history-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  border-bottom: 1px solid #222233;
  font-size: 12px;
  color: #8888a0;
}
.history-clear {
  background: none;
  border: none;
  color: #6c8ee3;
  font-size: 12px;
  cursor: pointer;
  padding: 2px 4px;
}
.history-clear:hover {
  color: #9bb5f0;
  text-decoration: underline;
}
.history-list {
  list-style: none;
  margin: 0;
  padding: 4px;
  max-height: 240px;
  overflow-y: auto;
}
.history-item {
  display: flex;
  align-items: center;
  border-radius: 6px;
  transition: background 0.12s;
}
.history-item:hover {
  background: rgba(255, 255, 255, 0.05);
}
.history-term {
  flex: 1;
  background: none;
  border: none;
  color: #d0d0dd;
  font-size: 13px;
  text-align: left;
  padding: 8px 10px;
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.history-remove {
  background: none;
  border: none;
  color: #666;
  font-size: 13px;
  cursor: pointer;
  padding: 8px 10px;
  transition: color 0.15s;
}
.history-remove:hover {
  color: #f87171;
}

/* 以图搜图横幅 */
.visual-target-banner {
  display: flex;
  align-items: center;
  gap: 12px;
  background: #0c0c12;
  border: 1px solid #2a2a3a;
  border-radius: 8px;
  padding: 10px 14px;
  margin-top: 15px;
}
.target-img {
  width: 64px;
  height: 64px;
  object-fit: cover;
  border-radius: 6px;
  border: 1px solid #333;
  flex-shrink: 0;
}
.target-info {
  flex: 1;
  min-width: 0;
}
.target-title {
  margin: 0 0 4px 0;
  font-size: 13px;
  font-weight: bold;
  color: #67e5e5;
}
.target-path {
  margin: 0;
  font-size: 11px;
  color: #888;
  word-break: break-all;
}
.target-note {
  margin: 2px 0 0;
  font-size: 11px;
  color: #ffaa33;
}

/* 条件面板 */
.cond-panel {
  margin-top: 15px;
  border: 1px solid rgba(108, 142, 227, 0.15);
  border-radius: 10px;
  padding: 12px;
  background: rgba(18, 18, 28, 0.4);
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.cond-panel-head {
  display: flex;
  align-items: center;
  gap: 10px;
}
.cond-group-label {
  font-size: 13px;
  color: #b0b0c0;
}
.cond-group-select {
  padding: 6px 10px;
  background: rgba(10, 10, 14, 0.6);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  color: #f0f0f5;
  font-size: 13px;
  cursor: pointer;
}
.cond-count {
  margin-left: auto;
  font-size: 12px;
  color: #8888a0;
}

.cond-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.cond-empty {
  margin: 0;
  font-size: 12px;
  color: #666677;
  padding: 6px 2px;
}

.cond-panel-foot {
  display: flex;
  align-items: center;
  gap: 10px;
  position: relative;
}
.add-cond-wrap {
  position: relative;
}
.btn-add-cond {
  padding: 6px 12px;
  background: rgba(108, 142, 227, 0.12);
  border: 1px dashed rgba(108, 142, 227, 0.4);
  border-radius: 6px;
  color: #93c5fd;
  font-size: 12px;
  cursor: pointer;
}
.add-cond-menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  min-width: 160px;
  background: #16161f;
  border: 1px solid #2a2a3a;
  border-radius: 8px;
  box-shadow: 0 12px 30px rgba(0, 0, 0, 0.5);
  z-index: 50;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.add-cond-item {
  background: none;
  border: none;
  color: #d0d0dd;
  font-size: 13px;
  text-align: left;
  padding: 8px 14px;
  cursor: pointer;
}
.add-cond-item:hover {
  background: rgba(108, 142, 227, 0.15);
}
.btn-clear-cond {
  padding: 6px 12px;
  background: none;
  border: none;
  color: #8888a0;
  font-size: 12px;
  cursor: pointer;
}
.btn-clear-cond:hover {
  color: #f87171;
}

.sort-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-top: 4px;
  border-top: 1px solid rgba(255, 255, 255, 0.05);
}
.sort-label {
  font-size: 12px;
  color: #8888a0;
}
.sort-main {
  font-size: 12px;
  color: #93c5fd;
}

.error-msg {
  color: #f87171;
  font-size: 12px;
  text-align: center;
  margin: 10px 0 0;
}
</style>
