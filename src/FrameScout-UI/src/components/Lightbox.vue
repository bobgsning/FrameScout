<script setup lang="ts">
/**
 * Lightbox — 大图预览 / 缩放 / 旋转 / 全屏 / 视频定位播放（P1-6 / P1-2 红框）。
 *
 * 第三轮修复：
 *   - 旧实现：ImageCard 是裸 <img> 无 @click，所有信息被压在 200px 卡片里。
 *   - 新实现：全屏暗背景 + 居中大图 + 左侧图 + 右侧 OCR 命中红框 SVG（P1-2）
 *     + 底部操作栏（打开/显示位置/以此为基准/复制路径/Space 下一张）。
 *   - 键盘：Space 下一张 / ←→ 翻 / Esc 关闭（由 useKeyboard 处理）。
 *   - OCR 红框：从 SearchResult.ocr_lines 取 bbox，画归一化 SVG 覆盖层。
 */
import { computed, ref, watch, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SearchResult } from '@/types/search'
import { getAssetUrl, isVideo } from '@/utils/media'
import { useFileActions } from '@/composables/useFileActions'
import { useToast } from '@/composables/useToast'

const props = defineProps<{
  visible: boolean
  items: SearchResult[]
  /** 当前索引 */
  index: number
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  close: []
  next: []
  prev: []
}>()

const { openFile, revealInExplorer, copyPath } = useFileActions()
const { push } = useToast()
const { t } = useI18n()

const zoom = ref(1)
const rotation = ref(0)
// 拖拽平移（pan）偏移，单位 px
const panX = ref(0)
const panY = ref(0)

const currentItem = computed(() => props.items[props.index] ?? null)
const isCurrentVideo = computed(() => currentItem.value ? isVideo(currentItem.value.path) : false)

// 重置缩放/旋转/平移
watch(() => props.index, resetView)
watch(() => props.visible, (v) => {
  if (v) resetView()
})

const imgTransform = computed(() => `translate(${panX.value}px, ${panY.value}px) scale(${zoom.value}) rotate(${rotation.value}deg)`)

function zoomIn() { zoom.value = Math.min(zoom.value * 1.25, 5) }
function zoomOut() { zoom.value = Math.max(zoom.value / 1.25, 0.2) }
function rotate() { rotation.value = (rotation.value + 90) % 360 }
function resetView() { zoom.value = 1; rotation.value = 0; panX.value = 0; panY.value = 0 }

// 拖拽平移：放大后才能拖拽移动观看（zoom=1 时图片居中，无需平移）
const isDragging = ref(false)
let dragStartX = 0
let dragStartY = 0
let panStartX = 0
let panStartY = 0

function onDragStart(e: MouseEvent) {
  if (isCurrentVideo.value || zoom.value <= 1) return
  isDragging.value = true
  dragStartX = e.clientX
  dragStartY = e.clientY
  panStartX = panX.value
  panStartY = panY.value
}
function onDragMove(e: MouseEvent) {
  if (!isDragging.value) return
  panX.value = panStartX + (e.clientX - dragStartX)
  panY.value = panStartY + (e.clientY - dragStartY)
}
function onDragEnd() {
  isDragging.value = false
}

// P0-6：滚轮缩放（此前提示文案承诺 "wheel=zoom" 但无 @wheel 监听，滚轮纹丝不动）
function onWheel(e: WheelEvent) {
  e.preventDefault()
  const factor = e.deltaY < 0 ? 1.1 : 1 / 1.1
  zoom.value = Math.min(Math.max(zoom.value * factor, 0.2), 5)
}

// P0-6：全屏（此前工具栏只有 reset view 的 ⤢，没有真正的 requestFullscreen）
const isFullscreen = ref(false)
async function toggleFullscreen() {
  try {
    if (!document.fullscreenElement) {
      await document.documentElement.requestFullscreen?.()
      isFullscreen.value = true
    } else {
      await document.exitFullscreen?.()
      isFullscreen.value = false
    }
  } catch {
    // 浏览器/环境不支持全屏时静默忽略
  }
}

// F 键全屏（window 级，仅 Lightbox 打开时生效）
function onWindowKeydown(e: KeyboardEvent) {
  if (!props.visible) return
  if (e.key === 'f' || e.key === 'F') {
    e.preventDefault()
    toggleFullscreen()
  }
}

watch(
  () => props.visible,
  (v) => {
    if (v) window.addEventListener('keydown', onWindowKeydown)
    else window.removeEventListener('keydown', onWindowKeydown)
  }
)
onUnmounted(() => window.removeEventListener('keydown', onWindowKeydown))

async function onCopyPath() {
  if (!currentItem.value) return
  const ok = await copyPath(currentItem.value.path)
  push(ok ? t('lightbox.pathCopied') : t('lightbox.copyFailed'), ok ? 'success' : 'error')
}

async function onOpen() {
  if (!currentItem.value) return
  await openFile(currentItem.value.path)
}

async function onReveal() {
  if (!currentItem.value) return
  await revealInExplorer(currentItem.value.path)
}

// OCR 命中行（P1-2 红框）
const ocrLines = computed(() => currentItem.value?.ocr_lines ?? [])
const hasOcrHits = computed(() => ocrLines.value.length > 0)
</script>

<template>
  <Teleport to="body">
    <div v-if="props.visible" class="lightbox-overlay" @click.self="emit('close')">
      <!-- 顶部操作栏 -->
      <div class="lb-topbar">
        <span class="lb-counter">{{ props.index + 1 }} / {{ props.items.length }}</span>
        <div class="lb-toolbar">
          <button class="lb-btn" :title="$t('lightbox.zoomIn')" @click="zoomIn">🔍+</button>
          <button class="lb-btn" :title="$t('lightbox.zoomOut')" @click="zoomOut">🔍−</button>
          <span class="lb-zoom-indicator">{{ Math.round(zoom * 100) }}%</span>
          <button class="lb-btn" :title="$t('lightbox.rotate')" @click="rotate">⟳</button>
          <button class="lb-btn" :title="$t('lightbox.reset')" @click="resetView">⤢</button>
          <button class="lb-btn" :title="$t('lightbox.fullscreen')" @click="toggleFullscreen">⛶</button>
          <span class="lb-divider" />
          <button class="lb-btn" :title="$t('lightbox.openFile')" @click="onOpen">📂</button>
          <button class="lb-btn" :title="$t('lightbox.reveal')" @click="onReveal">🗂️</button>
          <button class="lb-btn" :title="$t('lightbox.copyPath')" @click="onCopyPath">📋</button>
          <span class="lb-divider" />
          <button class="lb-btn" :title="$t('lightbox.prev')" @click="emit('prev')">←</button>
          <button class="lb-btn" :title="$t('lightbox.next')" @click="emit('next')">→</button>
          <button class="lb-btn lb-close" :title="$t('lightbox.close')" @click="emit('close')">✕</button>
        </div>
      </div>

      <!-- 主体：左图右 OCR 红框 -->
      <div class="lb-body">
        <div
          class="lb-media-area"
          :class="{ 'is-dragging': isDragging }"
          @wheel.prevent="onWheel"
          @mousedown="onDragStart"
          @mousemove="onDragMove"
          @mouseup="onDragEnd"
          @mouseleave="onDragEnd"
        >
          <div class="lb-stage">
            <video
              v-if="isCurrentVideo"
              :src="getAssetUrl(currentItem?.path || '')"
              controls
              autoplay
              class="lb-media"
              :style="{ transform: imgTransform }"
            />
            <img
              v-else
              :src="getAssetUrl(currentItem?.path || '')"
              class="lb-media"
              :style="{ transform: imgTransform }"
            />
            <!-- OCR 命中红框（P1-2 / 债单 A8）：按归一化 bbox 画 SVG 覆盖层。
                 仅对图片生效；视频的 OCR 命中在右侧文字列表展示（帧级定位需逐帧 overlay）。 -->
            <svg
              v-if="hasOcrHits && !isCurrentVideo"
              class="lb-ocr-overlay"
              viewBox="0 0 1 1"
              preserveAspectRatio="none"
            >
              <rect
                v-for="(line, i) in ocrLines"
                :key="i"
                :x="line.bbox_left"
                :y="line.bbox_top"
                :width="Math.max(0, line.bbox_right - line.bbox_left)"
                :height="Math.max(0, line.bbox_bottom - line.bbox_top)"
                class="lb-ocr-rect"
              />
            </svg>
          </div>
        </div>

        <!-- OCR 命中红框（P1-2） -->
        <div v-if="hasOcrHits" class="lb-ocr-panel">
          <p class="lb-ocr-title">{{ $t('lightbox.matches', { count: ocrLines.length }) }}</p>
          <div class="lb-ocr-list">
            <div
              v-for="(line, i) in ocrLines"
              :key="i"
              class="lb-ocr-line"
            >
              <span class="lb-ocr-conf">{{ (line.conf * 100).toFixed(0) }}%</span>
              <span class="lb-ocr-text">{{ line.text }}</span>
              <span class="lb-ocr-ts" v-if="line.timestamp > 0">{{ $t('lightbox.atTime', { time: line.timestamp.toFixed(1) }) }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 底部提示 -->
      <div class="lb-footer">
        <span class="lb-hint">{{ $t('lightbox.hints') }}</span>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.lightbox-overlay {
  position: fixed;
  inset: 0;
  z-index: 9998;
  background: rgba(0, 0, 0, 0.92);
  display: flex;
  flex-direction: column;
}

.lb-topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 24px;
  background: linear-gradient(180deg, rgba(0,0,0,0.6), transparent);
}

.lb-counter {
  color: #8888a0;
  font-size: 14px;
  font-weight: 500;
}

.lb-toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
}

.lb-btn {
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  color: #f0f0f5;
  font-size: 14px;
  cursor: pointer;
  padding: 6px 10px;
  transition: all 0.15s;
  min-width: 36px;
}

.lb-btn:hover {
  background: rgba(108, 142, 227, 0.2);
  border-color: rgba(108, 142, 227, 0.4);
}

.lb-zoom-indicator {
  min-width: 44px;
  text-align: center;
  font-size: 12px;
  color: #8888a0;
  font-variant-numeric: tabular-nums;
}

.lb-close:hover {
  background: rgba(248, 113, 113, 0.2);
  border-color: rgba(248, 113, 113, 0.4);
}

.lb-divider {
  width: 1px;
  height: 20px;
  background: rgba(255, 255, 255, 0.1);
  margin: 0 6px;
}

.lb-body {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.lb-media-area {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: auto;
  position: relative;
}

.lb-stage {
  position: relative;
  line-height: 0;
  flex-shrink: 0;
}

.lb-media {
  display: block;
  max-width: 88vw;
  max-height: 76vh;
  width: auto;
  height: auto;
  object-fit: contain;
  transition: transform 0.15s ease;
  border-radius: 4px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  cursor: grab;
  user-select: none;
  -webkit-user-drag: none;
}
.lb-media-area.is-dragging .lb-media {
  transition: none;
  cursor: grabbing;
}

.lb-ocr-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

.lb-ocr-rect {
  fill: rgba(248, 113, 113, 0.12);
  stroke: #f87171;
  stroke-width: 2;
  vector-effect: non-scaling-stroke;
}

.lb-ocr-panel {
  width: 320px;
  background: rgba(20, 20, 30, 0.95);
  border-left: 1px solid rgba(108, 142, 227, 0.2);
  padding: 16px;
  overflow-y: auto;
  flex-shrink: 0;
}

.lb-ocr-title {
  margin: 0 0 12px 0;
  color: #6c8ee3;
  font-size: 14px;
  font-weight: 600;
}

.lb-ocr-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.lb-ocr-line {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  background: rgba(108, 142, 227, 0.08);
  border-radius: 6px;
  border-left: 3px solid #6c8ee3;
}

.lb-ocr-conf {
  font-size: 11px;
  color: #4ade80;
  font-weight: 600;
  flex-shrink: 0;
  min-width: 32px;
}

.lb-ocr-text {
  flex: 1;
  font-size: 12px;
  color: #f0f0f5;
  word-break: break-word;
}

.lb-ocr-ts {
  font-size: 11px;
  color: #8888a0;
  flex-shrink: 0;
}

.lb-footer {
  padding: 8px 24px;
  text-align: center;
  background: linear-gradient(0deg, rgba(0,0,0,0.6), transparent);
}

.lb-hint {
  font-size: 11px;
  color: #555566;
}
</style>
