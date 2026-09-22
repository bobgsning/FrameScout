<script setup lang="ts">
import { computed } from 'vue'
import type { SearchResult } from '@/types/search'
import { highlight } from '@/utils/highlight'
import { runOcrForItem } from '@/utils/api'
import { useScanner } from '@/composables/useScanner'

const props = defineProps<{
  item: SearchResult
  searchQuery: string
  isImageSearch: boolean
}>()

// 卡片未单独指定语言时，回退到扫描栏的全局 OCR 语言设置
const { ocrLanguages } = useScanner()

const highlighted = computed(() =>
  highlight(props.item.ocr_text, props.searchQuery, props.isImageSearch)
)
</script>

<template>
  <div v-if="props.item.ocr_text" class="ocr-panel">
    <p class="ocr-label">🔍 Extracted Text:</p>
    <p class="ocr-content" :class="{ 'ocr-collapsed': !props.item.expandOcr }" v-html="highlighted"></p>
    <button
      v-if="props.item.ocr_text.length > 50"
      class="btn-text"
      @click="props.item.expandOcr = !props.item.expandOcr"
    >
      {{ props.item.expandOcr ? 'Collapse 🔼' : 'Expand 🔽' }}
    </button>
  </div>

  <!-- 尚无 OCR 文本时，提供即时识别入口 -->
  <div v-else class="ocr-run-panel">
    <button
      class="btn btn-sm btn-outline-primary"
      :disabled="props.item.ocrRunning"
      @click="runOcrForItem(props.item, ocrLanguages)"
    >
      {{ props.item.ocrRunning ? '⏳ Running...' : '💬 Run OCR' }}
    </button>
    <input
      v-model="props.item.ocrLangInput"
      type="text"
      placeholder="lang (e.g. en, ch_sim)"
      class="ocr-lang-input-sm"
    />
  </div>
</template>
