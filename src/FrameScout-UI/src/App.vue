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
      @clean-ghosts="cleanGhosts"
    />

    <SmartFolderBar
      :smart-folders="smartFolders"
      @apply="applySmartFolder"
      @remove="removeSmartFolder"
    />

    <ExtractionBus
      :is-scanning="isScanning"
      :current-status="currentStatus"
      :current-file="currentFile"
      :scan-progress="scanProgress"
      :scan-msg="scanMsg"
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
      @recluster="fetchClusters"
    />

    <div v-else>
      <SearchConsole
        v-model:query="searchQuery"
        v-model:page-size="pageSize"
        v-model:s-filename="s_filename"
        v-model:s-note="s_note"
        v-model:s-ocr="s_ocr"
        v-model:s-vector="s_vector"
        :is-image-search="isImageSearch"
        :search-image-path="searchImagePath"
        :search-msg="searchMsg"
        @search="resetAndSearch"
        @search-by-image="searchByImageAction"
        @clear-image-search="clearImageSearch"
        @save-smart-folder="promptSaveSmartFolder"
        @show-all="showAllFiles"
      />

      <div class="results-grid">
        <ResultCard
          v-for="item in results"
          :key="item.path"
          :item="item"
          :search-query="searchQuery"
          :is-image-search="isImageSearch"
        />
      </div>

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
  </div>
</template>

<script setup lang="ts">
// ---- 业务逻辑切片（全部为模块级单例，此处只是取引用） ----
import { useEngineStatus } from '@/composables/useEngineStatus'
import { useLicense } from '@/composables/useLicense'
import { useSmartFolders } from '@/composables/useSmartFolders'
import { useScanner } from '@/composables/useScanner'
import { useSearch } from '@/composables/useSearch'
import { useClustering } from '@/composables/useClustering'

// ---- 视图组件 ----
import SvgDefs from '@/components/common/SvgDefs.vue'
import SplashScreen from '@/components/splash/SplashScreen.vue'
import BrandHeader from '@/components/header/BrandHeader.vue'
import LicenseModal from '@/components/header/LicenseModal.vue'
import TopActionBar from '@/components/scan/TopActionBar.vue'
import ExtractionBus from '@/components/scan/ExtractionBus.vue'
import IncomingBanner from '@/components/scan/IncomingBanner.vue'
import SmartFolderBar from '@/components/smart-folders/SmartFolderBar.vue'
import ClusterView from '@/components/cluster/ClusterView.vue'
import SearchConsole from '@/components/search/SearchConsole.vue'
import ResultCard from '@/components/cards/ResultCard.vue'
import PaginationBar from '@/components/search/PaginationBar.vue'

// 调用顺序即生命周期注册顺序：引擎握手 -> 授权 -> 数据 -> 视图状态
const { engineReady, engineStatus, engineMessage, engineRetry, engineMaxRetries } = useEngineStatus()
const { licenseStatus, showActivateModal } = useLicense()
const { smartFolders, promptSaveSmartFolder, removeSmartFolder, applySmartFolder } =
  useSmartFolders()
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
  showIncomingBanner,
  incomingCount,
  startScan,
  selectAndIndexFiles,
  cleanGhosts,
  acceptIncomingFiles,
  dismissIncomingBanner
} = useScanner()
const {
  searchQuery,
  isImageSearch,
  searchImagePath,
  results,
  s_filename,
  s_note,
  s_ocr,
  s_vector,
  pageSize,
  currentPage,
  totalResults,
  totalPages,
  isShowAllMode,
  searchMsg,
  resetAndSearch,
  searchByImageAction,
  clearImageSearch,
  changePage,
  doJump,
  showAllFiles,
  exitShowAllMode
} = useSearch()
const { isClusteringView, clusterThreshold, clusters, toggleClustering, fetchClusters } =
  useClustering()
</script>
