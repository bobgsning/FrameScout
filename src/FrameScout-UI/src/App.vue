<!--
  SPDX-License-Identifier: Apache-2.0
  FrameScout — Offline AI Search — Vue 3 Frontend
  =============================
  Top-level orchestrator: 只负责组装 composables 与组件，不含业务逻辑。
  （原 700+ 行单文件已按 types / utils / styles / composables / components 拆分）

  Copyright (c) 2026 AetherFlow Labs Inc.
-->
<template>
  <!-- 1. 全局 SVG 资产库（常驻 DOM） -->
  <SvgDefs />

  <!-- 2. 启动握手等待屏 -->
  <SplashScreen
    v-if="!engineReady"
    :status="engineStatus"
    :message="engineMessage"
    :retry="engineRetry"
    :max-retries="engineMaxRetries"
  />

  <!-- 3. 主应用容器 -->
  <div v-else class="app-container">
    <BrandHeader :is-pro="licenseStatus.is_pro" @open-license="showActivateModal = true" />

    <LicenseModal
      v-if="showActivateModal"
      :license-status="licenseStatus"
      @close="showActivateModal = false"
    />

    <TopActionBar
      v-model:folder-path="folderPath"
      v-model:scan-mode="scanMode"
      v-model:enable-ocr="enableOcr"
      v-model:ocr-languages="ocrLanguages"
      :is-scanning="isScanning"
      :is-clustering-view="isClusteringView"
      @start-scan="startScan"
      @index-files="selectAndIndexFiles"
      @toggle-clustering="toggleClustering"
      @show-reports="openScanReports"
      @review-changes="openDiffReport"
      @clean-ghosts="openGhostCleanup"
      @manage-text="openManager"
      @insert-text="textEntryDialogOpen = true"
      @show-all="showAllFiles"
      @open-settings="settingsOpen = true"
      @open-timeline="timelineOpen = true"
    />

    <!-- 扫描报告 / 活动记录：读 batches 表，重启不丢 -->
    <ScanReportDialog
      :open="reportDialogOpen"
      :reports="scanReports"
      :message="reportMsg"
      @close="closeScanReports"
    />

    <!-- 差异报告：只感知，行动由用户勾选后触发 -->
    <DiffReportDialog
      :open="diffDialogOpen"
      :report="diffReport"
      :busy="diffBusy"
      :message="diffMsg"
      @close="closeDiffReport"
      @index="indexDiffSelection"
      @mark-dead="markDeadDiffSelection"
      @ignore="ignoreDiffSelection"
    />

    <!-- 幽灵清理：先预览、后行动 -->
    <GhostCleanupDialog
      :open="ghostDialogOpen"
      :items="ghostItems"
      :busy="ghostBusy"
      :message="ghostMsg"
      @close="closeGhostCleanup"
      @apply="applyGhostAction"
    />

    <SmartFolderBar
      :smart-folders="smartFolders"
      @apply="applySmartFolder"
      @remove="removeSmartFolder"
      @refresh-count="refreshFolderCount"
      @edit="onEditFolder"
    />

    <ExtractionBus
      :is-scanning="isScanning"
      :current-status="currentStatus"
      :current-file="currentFile"
      :scan-progress="scanProgress"
      :scan-msg="scanMsg"
      :scan-elapsed="scanElapsed"
      :scan-eta="scanEta"
      @stop="stopScan"
    />

    <IncomingBanner
      v-if="showIncomingBanner && incomingCount > 0"
      :count="incomingCount"
      @accept="acceptIncomingFiles"
      @dismiss="dismissIncomingBanner"
    />

    <!-- 视图切换：聚类视图 vs 搜索/浏览视图 -->
    <ClusterView
      v-if="isClusteringView"
      v-model:threshold="clusterThreshold"
      :clusters="clusters"
      :truncated-single-frames="truncatedSingleFrames"
      @recluster="fetchClusters"
      @open-preview="openClusterPreview"
    />

    <div v-else>
      <SearchConsole
        v-model:query="searchQuery"
        v-model:page-size="pageSize"
        v-model:s-filename="s_filename"
        v-model:s-note="s_note"
        v-model:s-ocr="s_ocr"
        v-model:s-vector="s_vector"
        v-model:s-text="s_text"
        v-model:search-mode="searchMode"
        :is-image-search="isImageSearch"
        :search-image-path="searchImagePath"
        :search-msg="searchMsg"
        @search="resetAndSearch"
        @search-by-image="searchByImageAction"
        @clear-image-search="clearImageSearch"
        @save-smart-folder="onSaveSmartFolder"
      />

      <!-- 纯文本命中：独立于媒体结果，展示内容本身（分页与媒体结果分开） -->
      <div v-if="textEntryResults.length > 0" class="results-grid">
        <TextEntryCard
          v-for="entry in textEntryResults"
          :key="entry.entry_id"
          :item="entry"
        />
      </div>

      <PaginationBar
        v-if="textEntryResults.length > 0"
        :current-page="textEntryPage"
        :total-pages="textEntryTotalPages"
        :total-results="textEntryTotal"
        :is-show-all-mode="false"
        :show-all-button="false"
        dense
        @change-page="changeTextEntryPage"
        @jump="jumpTextEntryPage"
      />

      <!-- P0-2 / P0-7：结果批量操作工具栏（全选本页 / 反选 / 常驻导出） -->
      <div v-if="results.length > 0" class="results-toolbar">
        <span class="results-toolbar-label">{{ $t('app.resultsCount', { count: results.length }) }}</span>
        <button
          class="btn btn-outline-primary btn-sm"
          @click="isAllPageSelected() ? invertPageSelection() : selectAllPage()"
        >
          {{ isAllPageSelected() ? $t('app.deselectAll') : $t('app.selectAll') }}
        </button>
        <button class="btn btn-outline-primary btn-sm" @click="invertPageSelection">
          {{ $t('app.invertSelection') }}
        </button>
        <span class="results-toolbar-divider" />
        <span class="results-toolbar-label">{{ $t('app.export') }}</span>
        <button class="btn btn-outline-primary btn-sm" :title="$t('tray.exportCsv')" @click="exportCurrentResults('csv')">📊 CSV</button>
        <button class="btn btn-outline-primary btn-sm" :title="$t('tray.exportMd')" @click="exportCurrentResults('md')">📝 MD</button>
        <button class="btn btn-outline-primary btn-sm" :title="$t('tray.exportJson')" @click="exportCurrentResults('json')">{ } JSON</button>
        <span class="results-toolbar-divider" />
        <span class="results-toolbar-label">{{ $t('app.sortBy') }}</span>
        <select v-model="sortBy" class="custom-select-sm" @change="applySort">
          <option value="relevance">{{ $t('app.sortRelevance') }}</option>
          <option value="name">{{ $t('app.sortName') }}</option>
          <option value="time">{{ $t('app.sortTime') }}</option>
        </select>
      </div>

      <!-- P1-2：批量加笔记进度 + 取消（此前串行 await 无进度无取消） -->
      <div v-if="batchNoteProgress.total > 0" class="batch-progress">
        <span class="batch-progress-label">
          {{ $t('app.batchNoteProgress', { done: batchNoteProgress.done, total: batchNoteProgress.total }) }}
        </span>
        <div class="batch-progress-track">
          <div
            class="batch-progress-fill"
            :style="{ width: (batchNoteProgress.total > 0 ? (batchNoteProgress.done / batchNoteProgress.total) * 100 : 0) + '%' }"
          ></div>
        </div>
        <button class="btn btn-outline-primary btn-sm" @click="cancelBatchNote">{{ $t('common.cancel') }}</button>
      </div>

      <!-- Show All 模式顶部提示条：与底部 PaginationBar 的 show-all-bar 一致，方便在长列表顶部一键退出分页视图 -->
      <div v-if="isShowAllMode && totalResults > 0" class="show-all-bar-top">
        <span class="show-all-info-top">{{ $t('pagination.showingAll', { total: totalResults }) }}</span>
        <button class="btn btn-secondary btn-sm" @click="exitShowAllMode">
          {{ $t('pagination.paginatedView') }}
        </button>
      </div>

      <div class="results-grid" @dragover.prevent="onDragOver" @drop.prevent="onDrop" @dragleave="onDragLeave">
        <!-- P1-7：搜索进行中骨架屏 -->
        <div v-if="isSearching" class="loading-skeleton">
          <div v-for="n in 8" :key="n" class="skeleton-card">
            <div class="skeleton-img"></div>
            <div class="skeleton-line"></div>
            <div class="skeleton-line short"></div>
          </div>
        </div>
        <!-- P1-7：空态引导 -->
        <div v-else-if="results.length === 0 && !searchMsg" class="empty-state">
          <p class="empty-icon">📂</p>
          <p class="empty-title">{{ isShowAllMode ? $t('app.emptyLibrary') : $t('app.noResults') }}</p>
          <p class="empty-hint" v-if="!isShowAllMode">{{ $t('app.emptyHintSearch') }}</p>
          <p class="empty-hint" v-else>{{ $t('app.emptyHintLibrary') }}</p>
        </div>
        <!-- 搜索错误提示 -->
        <div v-else-if="searchMsg && results.length === 0" class="empty-state">
          <p class="empty-icon">⚠️</p>
          <p class="empty-title">{{ searchMsg }}</p>
        </div>
        <!-- 正常结果列表（用 template 包裹 v-else + v-for，避免 Vue 3 优先级冲突） -->
        <template v-else>
          <ResultCard
            v-for="item in results"
            :key="item.path"
            :item="item"
            :search-query="searchQuery"
            :is-image-search="isImageSearch"
            @open-lightbox="openLightbox"
          @search-by-image="searchByImageAction"
          @add-to-selection="onAddToSelection"
          />
          </template>
          </div>

      <!-- 拖拽落区高亮 -->
      <div v-if="dragActive" class="drop-zone-hint">
        <p>{{ $t('app.dropHint') }}</p>
      </div>

      <TextEntryDialog
        :open="textEntryDialogOpen"
        :text="textEntryInput"
        :source="textEntrySource"
        @update:text="textEntryInput = $event"
        @update:source="textEntrySource = $event"
        @close="textEntryDialogOpen = false"
        @save="insertTextEntry"
      />

      <!-- 纯文本管理：先预览（列出条目）→ 用户勾选 → 才允许删除或导入 -->
      <TextEntryManagerDialog
        :open="managerOpen"
        :entries="textEntries"
        :total="textEntriesTotal"
        :page="textEntriesPage"
        :total-pages="textEntriesTotalPages"
        :page-size="textEntriesPageSize"
        :query="textEntriesQuery"
        :busy="textEntriesBusy"
        :message="textEntriesMsg"
        :selected="selectedEntryIds"
        @close="closeManager"
        @toggle="toggleEntry"
        @select-all-page="toggleSelectAllPage"
        @update:page-size="changeTextEntriesPageSize"
        @delete="deleteSelected"
        @update:query="textEntriesQuery = $event"
        @search="searchTextEntries"
        @change-page="changeTextEntriesPage"
        @jump="jumpTextEntriesPage"
        @pick-files="pickTextFiles"
        @pick-folder="pickTextFolder"
      />

      <PaginationBar
        v-if="totalResults > 0"
        :current-page="currentPage"
        :total-pages="totalPages"
        :total-results="totalResults"
        :is-show-all-mode="isShowAllMode"
        @change-page="changePage"
        @jump="doJump"
        @show-all="showAllFiles"
        @exit-show-all="exitShowAllMode"
      />
    </div>

    <!-- 智能文件夹命名对话框（P1-3：取消原生 prompt） -->
    <PromptDialog
      v-model:visible="showSmartFolderNaming"
      :title="$t('app.smartFolderDialogTitle')"
      :label="$t('app.folderNameLabel')"
      :placeholder="$t('app.folderNamePlaceholder')"
      :default-value="searchQuery"
      :max-length="50"
      :confirm-text="$t('common.save')"
      @confirm="onConfirmSmartFolderName"
    />

    <!-- 智能文件夹重命名对话框（P1-1：接 updateSmartFolder） -->
    <PromptDialog
      v-model:visible="showEditFolderNaming"
      :title="$t('app.editSmartFolderTitle')"
      :label="$t('app.folderNameLabel')"
      :placeholder="$t('app.folderNamePlaceholder')"
      :default-value="editFolder?.name || ''"
      :max-length="50"
      :confirm-text="$t('common.save')"
      @confirm="onConfirmEditFolderName"
    />

    <!-- 批量加笔记对话框（P1-10 / 债单 A10：从 toast 占位到真实现） -->
    <PromptDialog
      v-model:visible="showBatchNote"
      :title="$t('app.batchNoteTitle')"
      :label="$t('app.noteContentLabel')"
      :placeholder="$t('app.notePlaceholder')"
      :max-length="200"
      :confirm-text="$t('app.saveNote')"
      multiline
      @confirm="onConfirmBatchNote"
    />

    <!-- 全局 Toast 通知（P1-8） -->
    <ToastContainer />

    <!-- Lightbox 大图预览（P1-6） -->
    <Lightbox
      :visible="lightboxVisible"
      :items="lightboxItems"
      :index="lightboxIndex"
      @close="lightboxVisible = false"
      @next="lightboxNext"
      @prev="lightboxPrev"
    />

    <!-- 候选集托盘（P1-10） -->
    <SelectionTray
      :results="results"
      @batch-add-note="onBatchAddNote"
      @add-to-smart-folder="onAddToSmartFolder"
    />

    <!-- 设置页（P1-5 / 债单 A2：孤儿组件接线） -->
    <SettingsDialog :visible="settingsOpen" @close="settingsOpen = false" />

    <!-- 时间轴（P2-4 / 债单 A3：孤儿组件接线） -->
    <TimelineView :visible="timelineOpen" @close="timelineOpen = false" />
  </div>
</template>

<script setup lang="ts">
// ---- 业务逻辑切片（全部为模块级单例，此处只是取引用） ----
import { useEngineStatus } from '@/composables/useEngineStatus'
import { useLicense } from '@/composables/useLicense'
import { useSmartFolders } from '@/composables/useSmartFolders'
import { useScanner } from '@/composables/useScanner'
import { useSearch } from '@/composables/useSearch'
import { useTextEntries } from '@/composables/useTextEntries'
import { useClustering } from '@/composables/useClustering'

// ---- 视图组件 ----
import SvgDefs from '@/components/common/SvgDefs.vue'
import SplashScreen from '@/components/splash/SplashScreen.vue'
import BrandHeader from '@/components/header/BrandHeader.vue'
import LicenseModal from '@/components/header/LicenseModal.vue'
import TopActionBar from '@/components/scan/TopActionBar.vue'
import ExtractionBus from '@/components/scan/ExtractionBus.vue'
import IncomingBanner from '@/components/scan/IncomingBanner.vue'
import GhostCleanupDialog from '@/components/scan/GhostCleanupDialog.vue'
import DiffReportDialog from '@/components/scan/DiffReportDialog.vue'
import ScanReportDialog from '@/components/scan/ScanReportDialog.vue'
import SmartFolderBar from '@/components/smart-folders/SmartFolderBar.vue'
import ClusterView from '@/components/cluster/ClusterView.vue'
import SearchConsole from '@/components/search/SearchConsole.vue'
import ResultCard from '@/components/cards/ResultCard.vue'
import TextEntryCard from '@/components/cards/TextEntryCard.vue'
import TextEntryDialog from '@/components/search/TextEntryDialog.vue'
import TextEntryManagerDialog from '@/components/search/TextEntryManagerDialog.vue'
import PaginationBar from '@/components/search/PaginationBar.vue'
import PromptDialog from '@/components/PromptDialog.vue'
import ToastContainer from '@/components/ToastContainer.vue'
import Lightbox from '@/components/Lightbox.vue'
import SelectionTray from '@/components/SelectionTray.vue'
import SettingsDialog from '@/components/SettingsDialog.vue'
import TimelineView from '@/components/TimelineView.vue'

import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useToast } from '@/composables/useToast'
import { useSelection } from '@/composables/useSelection'
import { useKeyboard, type CloseableDialog } from '@/composables/useKeyboard'
import { saveNote } from '@/utils/api'
import { exportCSV, exportMarkdown, exportJSON } from '@/utils/exporters'
import type { SearchResult, SmartFolder } from '@/types/search'

const { t } = useI18n()

// 调用顺序即生命周期注册顺序：引擎握手 -> 授权 -> 数据 -> 视图状态
const { engineReady, engineStatus, engineMessage, engineRetry, engineMaxRetries } = useEngineStatus()
const { licenseStatus, showActivateModal } = useLicense()
const { smartFolders, saveSmartFolder, removeSmartFolder, applySmartFolder, refreshFolderCount, updateSmartFolder } =
  useSmartFolders()

// 智能文件夹命名对话框（P1-3：取消原生 prompt，改用应用内对话框）
const showSmartFolderNaming = ref(false)
function onSaveSmartFolder() {
  showSmartFolderNaming.value = true
}

// P1-1：智能文件夹重命名（updateSmartFolder 此前零调用方，后端已就绪）
const showEditFolderNaming = ref(false)
const editFolder = ref<SmartFolder | null>(null)
function onEditFolder(folder: SmartFolder) {
  editFolder.value = folder
  showEditFolderNaming.value = true
}
function onConfirmEditFolderName(name: string) {
  if (!editFolder.value) return
  updateSmartFolder(
    editFolder.value.id,
    name,
    editFolder.value.query_text,
    editFolder.value.use_vector,
    editFolder.value.use_ocr,
    editFolder.value.use_note,
    editFolder.value.use_filename
  )
  editFolder.value = null
}

// ---- M0'：设置页 + 时间轴入口状态（孤儿组件接线） ----
const settingsOpen = ref(false)
const timelineOpen = ref(false)
function onConfirmSmartFolderName(name: string) {
  saveSmartFolder(name)
}

// ---- P1-6 Lightbox 大图预览 ----
const lightboxVisible = ref(false)
const lightboxItems = ref<SearchResult[]>([])
const lightboxIndex = ref(0)

function openLightbox(item: SearchResult) {
  // 用当前结果列表作为 Lightbox 的浏览序列
  lightboxItems.value = results.value
  lightboxIndex.value = results.value.findIndex((r) => r.path === item.path)
  if (lightboxIndex.value < 0) lightboxIndex.value = 0
  lightboxVisible.value = true
}
function lightboxNext() {
  if (lightboxIndex.value < lightboxItems.value.length - 1) {
    lightboxIndex.value++
  }
}
function lightboxPrev() {
  if (lightboxIndex.value > 0) {
    lightboxIndex.value--
  }
}

// 聚类缩略图预览：把簇内路径构造成最小 SearchResult 序列，复用 Lightbox 浏览
function openClusterPreview(paths: string[], index: number) {
  lightboxItems.value = paths.map((p) => ({
    path: p,
    timestamp: 0,
    score: 0,
    matched_tags: [],
    ocr_text: '',
    user_note: '',
  }))
  lightboxIndex.value = Math.max(0, Math.min(index, paths.length - 1))
  lightboxVisible.value = true
}

// ---- P1-7 拖拽即搜 ----
const dragActive = ref(false)

function onDragOver() {
  dragActive.value = true
}
function onDragLeave() {
  dragActive.value = false
}
async function onDrop(e: DragEvent) {
  dragActive.value = false
  if (!e.dataTransfer) return
  // 取第一个文件作为以图搜图的种子
  const files = Array.from(e.dataTransfer.files)
  if (files.length === 0) return
  const file = files[0]
  // 用 file.path（Tauri webview 支持）或 fallback 到 file
  const path = (file as any).path || ''
  if (path) {
    // P1-7 拖拽即搜：直接设置路径并触发以图搜图
    searchImagePath.value = path
    isImageSearch.value = true
    searchQuery.value = ''
    currentPage.value = 1
    jumpPage.value = 1
    performSearch()
  } else {
    const { push } = useToast()
    push(t('app.dropPathError'), 'warning')
  }
}

// ---- P1-10 多选 + 候选集 ----
const { toggle: toggleSelection, isSelected, selectMany, rememberResult } = useSelection()

function onAddToSelection(path: string) {
  toggleSelection(path)
}

// P0-2：全选本页 / 反选（此前无勾选框，selectMany 零调用方）
function selectAllPage() {
  for (const r of results.value) rememberResult(r)
  selectMany(results.value.map((r) => r.path))
}
function invertPageSelection() {
  for (const r of results.value) {
    rememberResult(r)
    toggleSelection(r.path)
  }
}
function isAllPageSelected(): boolean {
  return results.value.length > 0 && results.value.every((r) => isSelected(r.path))
}

// P2-3：结果排序（前端当前页排序；相关度=后端顺序，另可按文件名/入库时间）
const sortBy = ref<'relevance' | 'name' | 'time'>('relevance')
function applySort() {
  const arr = [...results.value]
  if (sortBy.value === 'name') {
    arr.sort((a, b) => a.path.localeCompare(b.path))
  } else if (sortBy.value === 'time') {
    arr.sort((a, b) => (b.index_time || 0) - (a.index_time || 0))
  } else {
    arr.sort((a, b) => b.score - a.score)
  }
  results.value = arr
}

// P0-7：结果区常驻导出（此前导出被三层隐藏：托盘 → 勾选 → 右键；现在不必先勾选）
function exportCurrentResults(format: 'csv' | 'md' | 'json') {
  const items = results.value
  if (items.length === 0) return
  const { push } = useToast()
  const stamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)
  if (format === 'csv') {
    exportCSV(items, `framescout_results_${stamp}.csv`)
    push(t('tray.exportedCsv', { count: items.length }), 'success')
  } else if (format === 'md') {
    exportMarkdown(items, `framescout_results_${stamp}.md`)
    push(t('tray.exportedMd', { count: items.length }), 'success')
  } else {
    exportJSON(items, `framescout_results_${stamp}.json`)
    push(t('tray.exportedJson', { count: items.length }), 'success')
  }
}

// ---- P1-10 / 债单 A10：批量加笔记（从 toast 占位到真实现） ----
const showBatchNote = ref(false)
const batchNotePaths = ref<string[]>([])
// P1-2：批量进度 + 取消 + 失败清单（此前串行 await，无进度无取消）
const batchNoteProgress = ref({ done: 0, total: 0 })
const batchNoteCancelled = ref(false)

function onBatchAddNote(paths: string[]) {
  if (paths.length === 0) {
    const { push } = useToast()
    push(t('app.selectFilesFirst'), 'warning')
    return
  }
  batchNotePaths.value = paths
  showBatchNote.value = true
}

function cancelBatchNote() {
  batchNoteCancelled.value = true
}

async function onConfirmBatchNote(note: string) {
  const { push } = useToast()
  const paths = batchNotePaths.value
  let okCount = 0
  let failCount = 0
  const failedPaths: string[] = []
  batchNoteCancelled.value = false
  batchNoteProgress.value = { done: 0, total: paths.length }
  for (const path of paths) {
    if (batchNoteCancelled.value) break
    // 文件级笔记：不传 timestamp，覆盖该文件所有帧（与单条笔记语义一致）
    const ok = await saveNote(path, note)
    if (ok) okCount++
    else {
      failCount++
      failedPaths.push(path)
    }
    batchNoteProgress.value = { done: batchNoteProgress.value.done + 1, total: paths.length }
  }
  const wasCancelled = batchNoteCancelled.value
  batchNoteProgress.value = { done: 0, total: 0 }

  if (wasCancelled) {
    push(t('app.noteAddedCancelled', { okCount, remaining: paths.length - okCount - failCount }), 'info')
  } else if (failCount === 0) {
    push(t('app.noteAdded', { count: okCount }), 'success')
  } else if (failedPaths.length > 0) {
    // 失败清单：展示前 3 个失败文件
    const sample = failedPaths.slice(0, 3).map((p) => p.split(/[\\/]/).pop()).join('、')
    push(t('app.noteAddedPartialWithSample', { okCount, failCount, sample }), 'warning')
  } else {
    push(t('app.noteAddedPartial', { okCount, failCount }), 'warning')
  }
}

// 加入智能文件夹：SelectionTray 已禁用该按钮（智能文件夹按查询动态匹配，无法手动加成员）
function onAddToSmartFolder(_paths: string[]) {
  // 不再处理，保留占位以兼容事件绑定
}

// ---- P1-7 首屏加载 ----
// onMounted 已由 useEngineStatus/useScanner 注册。
// 引擎就绪后，若无搜索词，拉一页浏览列表。
// 注：useSearch.loadBrowsePage 已在 Browse 模式下由其他路径间接触发，
// 这里不重复调用以避免与 Splash 流程冲突。
const {
  folderPath,
  scanMode,
  enableOcr,
  ocrLanguages,
  isScanning,
  scanMsg,
  currentStatus,
  currentFile,
  scanProgress,
  scanElapsed,
  scanEta,
  showIncomingBanner,
  incomingCount,
  ghostItems,
  ghostDialogOpen,
  ghostBusy,
  ghostMsg,
  diffReport,
  diffDialogOpen,
  diffBusy,
  diffMsg,
  scanReports,
  reportDialogOpen,
  reportMsg,
  startScan,
  stopScan,
  openScanReports,
  closeScanReports,
  selectAndIndexFiles,
  openDiffReport,
  closeDiffReport,
  indexDiffSelection,
  markDeadDiffSelection,
  ignoreDiffSelection,
  openGhostCleanup,
  closeGhostCleanup,
  applyGhostAction,
  acceptIncomingFiles,
  dismissIncomingBanner
} = useScanner()
const {
  searchQuery,
  isImageSearch,
  searchImagePath,
  results,
  isSearching,
  s_filename,
  s_note,
  s_ocr,
  s_vector,
  s_text,
  searchMode,
  textEntryResults,
  textEntryPage,
  textEntryTotal,
  textEntryTotalPages,
  textEntryDialogOpen,
  textEntryInput,
  textEntrySource,
  pageSize,
  currentPage,
  totalResults,
  totalPages,
  jumpPage,
  isShowAllMode,
  searchMsg,
  resetAndSearch,
  performSearch,
  searchByImageAction,
  clearImageSearch,
  changePage,
  doJump,
  showAllFiles,
  exitShowAllMode,
  insertTextEntry,
  changeTextEntryPage,
  jumpTextEntryPage
} = useSearch()
const {
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
  openManager,
  closeManager,
  toggleEntry,
  toggleSelectAllPage,
  changeTextEntriesPageSize,
  deleteSelected,
  searchTextEntries,
  changeTextEntriesPage,
  jumpTextEntriesPage,
  pickTextFiles,
  pickTextFolder
} = useTextEntries()
const { isClusteringView, clusterThreshold, clusters, truncatedSingleFrames, toggleClustering, fetchClusters } =
  useClustering()

// ---- P0-1：全局键盘快捷键（useKeyboard 接线，此前是孤儿，导致弹窗无法 Esc） ----
// 收集当前打开的弹窗，按打开顺序压栈（数组末尾 = 最上层），Esc 关闭栈顶。
// 注：PromptDialog / ConfirmDialog / LicenseModal 各自已有键盘处理，不纳入栈，避免 Esc 双触发。
const dialogStack = computed<CloseableDialog[]>(() => {
  const stack: CloseableDialog[] = []
  if (reportDialogOpen.value) stack.push({ name: 'scan-report', close: closeScanReports })
  if (diffDialogOpen.value) stack.push({ name: 'diff-report', close: closeDiffReport })
  if (ghostDialogOpen.value) stack.push({ name: 'ghost-cleanup', close: closeGhostCleanup })
  if (textEntryDialogOpen.value) stack.push({ name: 'text-entry', close: () => { textEntryDialogOpen.value = false } })
  if (managerOpen.value) stack.push({ name: 'text-manager', close: closeManager })
  if (settingsOpen.value) stack.push({ name: 'settings', close: () => { settingsOpen.value = false } })
  if (timelineOpen.value) stack.push({ name: 'timeline', close: () => { timelineOpen.value = false } })
  if (lightboxVisible.value) stack.push({ name: 'lightbox', close: () => { lightboxVisible.value = false } })
  return stack
})

function focusSearch() {
  document.getElementById('searchQuery')?.focus()
}

useKeyboard(() => dialogStack.value, {
  focusSearch,
  prevPage: () => changePage(-1),
  nextPage: () => changePage(1),
  lightboxNext,
  lightboxPrev,
  isLightboxOpen: () => lightboxVisible.value
})
</script>
