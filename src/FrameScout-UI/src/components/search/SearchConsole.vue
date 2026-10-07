<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { getAssetUrl } from '@/utils/media'
import { usePreferences } from '@/composables/usePreferences'
import type { SearchMode } from '@/types/search'

const props = defineProps<{
  query: string
  pageSize: number
  sFilename: boolean
  sNote: boolean
  sOcr: boolean
  sVector: boolean
  sText: boolean
  searchMode: SearchMode
  isImageSearch: boolean
  searchImagePath: string
  searchMsg: string
}>()

const emit = defineEmits<{
  (e: 'update:query', value: string): void
  (e: 'update:pageSize', value: number): void
  (e: 'update:sFilename', value: boolean): void
  (e: 'update:sNote', value: boolean): void
  (e: 'update:sOcr', value: boolean): void
  (e: 'update:sVector', value: boolean): void
  (e: 'update:sText', value: boolean): void
  (e: 'update:searchMode', value: SearchMode): void
  (e: 'search'): void
  (e: 'search-by-image'): void
  (e: 'clear-image-search'): void
  (e: 'save-smart-folder'): void
}>()

/**
 * 目标通道定义：搜什么。
 * `enabled` 随模式变化——不适用于当前模式的目标置灰（系统有感知，但不替用户做决定，
 * 只是把「这个模式能搜什么」摆清楚）。纯文本（text）仅在 BGE-M3 模式下可用。
 */
type TargetKey = 'filename' | 'note' | 'ocr' | 'image' | 'text'

interface TargetDef {
  key: TargetKey
  label: string
  checked: boolean
  enabled: boolean
  pending?: boolean
}

const { t } = useI18n()

const targets = computed<TargetDef[]>(() => {
  const mode = props.searchMode
  return [
    // 模糊（semantic）下文字目标也可勾选：视觉语义 + 文字模糊匹配可组合，绝不锁死
    { key: 'filename', label: t('search.filename'), checked: props.sFilename, enabled: mode === 'exact' || mode === 'semantic' },
    { key: 'note', label: t('search.note'), checked: props.sNote, enabled: mode === 'exact' || mode === 'semantic' },
    { key: 'ocr', label: t('search.ocr'), checked: props.sOcr, enabled: mode === 'exact' || mode === 'semantic' || mode === 'bge' },
    { key: 'image', label: t('search.image'), checked: props.sVector, enabled: mode === 'semantic' },
    { key: 'text', label: t('search.text'), checked: props.sText, enabled: mode === 'bge' }
  ]
})

function onModeChange(mode: SearchMode) {
  emit('update:searchMode', mode)
  emit('search')
}

function onTargetChange(key: TargetKey, e: Event) {
  const checked = (e.target as HTMLInputElement).checked
  const emitKey: Record<TargetKey, string | null> = {
    filename: 'sFilename',
    note: 'sNote',
    ocr: 'sOcr',
    image: 'sVector',
    text: 'sText'
  }
  const keyName = emitKey[key]
  if (keyName) {
    emit(`update:${keyName}` as never, checked)
    emit('search')
  }
}

function onPageSizeChange(e: Event) {
  const value = Number((e.target as HTMLSelectElement).value)
  emit('update:pageSize', value)
  emit('search')
}

// ---------- P0-3：搜索历史（此前四件套零调用，从未上线） ----------
const { searchHistory, removeSearchHistoryEntry, clearSearchHistory } = usePreferences()
const historyOpen = ref(false)

function onSearchFocus() {
  if (searchHistory.value.length > 0) historyOpen.value = true
}
function onSearchBlur() {
  // 延迟隐藏，让用户能点到历史项 / 单条 ✕
  setTimeout(() => { historyOpen.value = false }, 150)
}
function pickHistory(q: string) {
  emit('update:query', q)
  emit('search')
  historyOpen.value = false
}
function removeHistory(q: string) {
  removeSearchHistoryEntry(q)
}
function clearHistory() {
  clearSearchHistory()
  historyOpen.value = false
}
</script>

<template>
  <div class="search-console">
    <div class="search-options">
      <!-- 模式：怎么匹配 -->
      <div class="mode-group">
        <span class="option-label">{{ $t('search.mode') }}</span>
        <div class="mode-segment">
          <button
            class="mode-btn"
            :class="{ active: props.searchMode === 'exact' }"
            :title="$t('search.exactHint')"
            @click="onModeChange('exact')"
          >
            {{ $t('search.exact') }}
          </button>
          <button
            class="mode-btn"
            :class="{ active: props.searchMode === 'semantic' }"
            :title="$t('search.visualHint')"
            @click="onModeChange('semantic')"
          >
            {{ $t('search.visual') }}
          </button>
          <button
            class="mode-btn"
            :class="{ active: props.searchMode === 'bge' }"
            :title="$t('search.textHint')"
            @click="onModeChange('bge')"
          >
            {{ $t('search.textSemantic') }}
          </button>
        </div>
      </div>

      <!-- 目标：搜什么 -->
      <div class="target-group">
        <span class="option-label">{{ $t('search.target') }}</span>
        <label
          v-for="t in targets"
          :key="t.key"
          class="target-item"
          :class="{ disabled: !t.enabled }"
          :title="t.pending ? $t('search.comingSoon') : !t.enabled ? $t('search.modeUnavailable') : ''"
        >
          <input
            type="checkbox"
            :checked="t.checked"
            :disabled="!t.enabled"
            @change="onTargetChange(t.key, $event)"
          />
          {{ t.label }}
        </label>
      </div>

      <!-- 每页条数 -->
      <div class="page-size-group">
        <span class="option-label">{{ $t('search.perPage') }}</span>
        <select :value="props.pageSize" @change="onPageSizeChange" class="custom-select-sm">
          <option :value="4">4</option>
          <option :value="8">8</option>
          <option :value="16">16</option>
          <option :value="32">32</option>
          <option :value="50">50</option>
          <option :value="100">100</option>
          <option :value="200">200</option>
        </select>
      </div>
    </div>

    <!-- 以图搜图的目标图预览 -->
    <div v-if="props.isImageSearch && props.searchImagePath" class="visual-target-banner">
      <img :src="getAssetUrl(props.searchImagePath)" class="target-img" />
      <div class="target-info">
        <p class="target-title">{{ $t('search.visualTarget') }}</p>
        <p class="target-path">{{ props.searchImagePath }}</p>
      </div>
      <button class="btn btn-danger btn-sm" @click="emit('clear-image-search')">{{ $t('search.cancel') }}</button>
    </div>

    <div class="search-bar">
      <div class="search-input-wrap">
        <input
          :value="props.query"
          @input="emit('update:query', ($event.target as HTMLInputElement).value)"
          @keyup.enter="emit('search')"
          @focus="onSearchFocus"
          @blur="onSearchBlur"
          id="searchQuery"
          type="text"
          :placeholder="$t('search.placeholder')"
          class="custom-input search-input"
        />
        <button
          v-if="props.query"
          class="clear-query"
          type="button"
          :title="$t('search.clearQuery')"
          @mousedown.prevent="emit('update:query', '')"
        >✕</button>

        <!-- 搜索历史下拉（P0-3） -->
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
      <button class="btn btn-primary search-btn" @click="emit('search')">{{ $t('search.search') }}</button>

      <button
        class="btn btn-outline-primary"
        :title="$t('search.similarHint')"
        @click="emit('search-by-image')"
      >
        {{ $t('search.similar') }}
      </button>

      <!-- 固定占位：Save Smart 只在有查询词时可用，但始终渲染以保持搜索栏宽度稳定 -->
      <button
        class="btn btn-secondary"
        :title="$t('search.saveSmartFolderHint')"
        :disabled="!props.query"
        @click="emit('save-smart-folder')"
      >
        {{ $t('search.save') }}
      </button>

      <!-- 搜索栏只保留「发起检索」相关的三个动作；
           浏览全部与纯文本录入属于「库维护」，已移到顶部探索与维护组。 -->
    </div>

    <p v-if="props.searchMsg" class="error-msg">{{ props.searchMsg }}</p>
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

.search-options {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 15px;
}

.mode-group,
.target-group,
.page-size-group {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.option-label {
  font-size: 12px;
  color: #888;
  min-width: 34px;
}

/* 分段按钮：模式三选一 */
.mode-segment {
  display: flex;
  border: 1px solid #333344;
  border-radius: 6px;
  overflow: hidden;
}
.mode-btn {
  padding: 6px 14px;
  background: #1e1e26;
  color: #aaa;
  border: none;
  cursor: pointer;
  font-size: 13px;
  white-space: nowrap;
  transition: all 0.15s ease;
}
.mode-btn:not(:last-child) {
  border-right: 1px solid #333344;
}
.mode-btn:hover {
  color: #fff;
}
.mode-btn.active {
  background: var(--grad-purple-cyan);
  color: #000;
  font-weight: bold;
}

/* 目标通道 checkbox */
.target-item {
  font-size: 13px;
  color: #ccc;
  cursor: pointer;
  white-space: nowrap;
}
.target-item.disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.search-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-wrap: wrap;
  gap: 10px;
}

/* 450px与480px刚刚好 */
.search-input {
  width: 100%;
  max-width: 450px;
  font-size: 16px;
  border-color: #8333ff;
}

/* 搜索历史下拉 + 输入框清除按钮 */
.search-input-wrap {
  position: relative;
  flex: 1 1 auto;
  max-width: 480px;
  min-width: 0;
}

/* 搜索 / 相似 / 保存按钮：窄窗口下不被输入框压缩或遮挡 */
.search-bar > button {
  flex-shrink: 0;
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

/* 以图搜图目标图横幅 */
.visual-target-banner {
  display: flex;
  align-items: center;
  gap: 12px;
  background: #0c0c12;
  border: 1px solid #2a2a3a;
  border-radius: 8px;
  padding: 10px 14px;
  margin-bottom: 15px;
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
</style>
