<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SearchResult } from '@/types/search'
import { isVideo } from '@/utils/media'
import { clearVideoTimer } from '@/utils/videoTimers'
import { useFileActions } from '@/composables/useFileActions'
import { useSelection } from '@/composables/useSelection'
import type { MenuItem } from '@/components/ContextMenu.vue'
import ContextMenu from '@/components/ContextMenu.vue'
import VideoCard from './VideoCard.vue'
import ImageCard from './ImageCard.vue'

const props = defineProps<{
  item: SearchResult
  searchQuery: string
  isImageSearch: boolean
}>()

const emit = defineEmits<{
  /** 以此为基准搜索（以图搜图种子） */
  searchByImage: [path: string]
  /** 加入候选集 */
  addToSelection: [path: string]
  /** 打开 Lightbox 大图预览 */
  openLightbox: [item: SearchResult]
}>()

const { openFile, revealInExplorer, copyPath } = useFileActions()
const { t } = useI18n()

// P0-2：可见勾选框（此前「选中」只能靠右键菜单，isSelected 零调用方）
const { isSelected, toggle: toggleSelection, rememberResult } = useSelection()
const selected = computed(() => isSelected(props.item.path))
function onToggleSelection() {
  rememberResult(props.item)
  toggleSelection(props.item.path)
}

const isVideoItem = computed(() => isVideo(props.item.path))

function handleVideoError() {
  clearVideoTimer(props.item.path)
  props.item.isMissing = true
}

function handleVideoLoaded() {
  clearVideoTimer(props.item.path)
}

// ---------- 右键上下文菜单（P1-1） ----------
const ctxVisible = ref(false)
const ctxX = ref(0)
const ctxY = ref(0)

const ctxItems = computed<MenuItem[]>(() => [
  { key: 'open', label: t('cards.openFile'), icon: '📂' },
  { key: 'reveal', label: t('cards.revealInExplorer'), icon: '🗂️' },
  { key: 'copyPath', label: t('cards.copyPath'), icon: '📋' },
  { key: 'searchByImage', label: t('cards.searchByThis'), icon: '🖼️' },
  { key: 'addToSelection', label: t('cards.addToCandidate'), icon: '⭐' },
  { key: 'lightbox', label: t('cards.openLarge'), icon: '🔍' },
])

function onContextMenu(e: MouseEvent) {
  e.preventDefault()
  ctxX.value = e.clientX
  ctxY.value = e.clientY
  ctxVisible.value = true
}

async function onCtxSelect(item: MenuItem) {
  switch (item.key) {
    case 'open':
      await openFile(props.item.path)
      break
    case 'reveal':
      await revealInExplorer(props.item.path)
      break
    case 'copyPath':
      await copyPath(props.item.path)
      break
    case 'searchByImage':
      emit('searchByImage', props.item.path)
      break
    case 'addToSelection':
      rememberResult(props.item)
      emit('addToSelection', props.item.path)
      break
    case 'lightbox':
      emit('openLightbox', props.item)
      break
  }
}

// 双击打开文件
function onDblClick() {
  openFile(props.item.path)
}
</script>

<template>
  <div
    class="result-card"
    :class="{ selected }"
    @contextmenu="onContextMenu"
    @dblclick="onDblClick"
  >
    <!-- P0-2：可见勾选框（不右键也能选中） -->
    <button
      class="selection-toggle"
      :class="{ checked: selected }"
      type="button"
      :title="selected ? $t('app.deselect') : $t('app.select')"
      @click.stop="onToggleSelection"
    >
      <span v-if="selected">✓</span>
    </button>

    <!-- 文件不可达（磁盘已删除 / 视频 25s 未加载出） -->
    <div v-if="props.item.isMissing" class="missing-card">
      <span class="missing-icon">👻</span>
      <span>{{ $t('cards.fileUnavailable') }}</span>
    </div>

    <VideoCard
      v-else-if="isVideoItem"
      :item="props.item"
      :search-query="props.searchQuery"
      :is-image-search="props.isImageSearch"
      @video-error="handleVideoError"
      @video-loaded="handleVideoLoaded"
    />

    <ImageCard
      v-else
      :item="props.item"
      :search-query="props.searchQuery"
      :is-image-search="props.isImageSearch"
      @open-lightbox="emit('openLightbox', $event)"
    />

    <ContextMenu
      :visible="ctxVisible"
      :items="ctxItems"
      :x="ctxX"
      :y="ctxY"
      @select="onCtxSelect"
      @close="ctxVisible = false"
    />
  </div>
</template>

<style scoped>
.result-card {
  position: relative;
  border-radius: 10px;
  transition: box-shadow 0.15s;
}
.result-card.selected {
  box-shadow: 0 0 0 2px #6c8ee3;
}

/* P0-2：可见勾选框（左上角，选中态高亮） */
.selection-toggle {
  position: absolute;
  top: 8px;
  left: 8px;
  z-index: 6;
  width: 24px;
  height: 24px;
  border-radius: 6px;
  border: 1.5px solid rgba(255, 255, 255, 0.4);
  background: rgba(0, 0, 0, 0.45);
  color: #fff;
  font-size: 14px;
  line-height: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s, transform 0.1s;
}
.selection-toggle:hover {
  border-color: rgba(255, 255, 255, 0.7);
  transform: scale(1.08);
}
.selection-toggle.checked {
  background: #6c8ee3;
  border-color: #6c8ee3;
}
</style>
