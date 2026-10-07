<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SearchResult } from '@/types/search'
import { highlight } from '@/utils/highlight'
import { runOcrForItem } from '@/utils/api'
import { useScanner } from '@/composables/useScanner'
import { useToast } from '@/composables/useToast'

const props = defineProps<{
  item: SearchResult
  searchQuery: string
  isImageSearch: boolean
}>()

// 卡片未单独指定语言时，回退到扫描栏的全局 OCR 语言设置
const { ocrLanguages } = useScanner()
const { push } = useToast()
const { t } = useI18n()

const highlighted = computed(() =>
  highlight(props.item.ocr_text, props.searchQuery, props.isImageSearch)
)

// P1-6：复制 OCR 文本（此前无法复制）
async function copyOcr() {
  try {
    await navigator.clipboard.writeText(props.item.ocr_text)
    push(t('cards.copied'), 'success')
  } catch {
    push(t('cards.copyFailed'), 'error')
  }
}
</script>

<template>
  <div v-if="props.item.ocr_text" class="ocr-panel">
    <div class="ocr-header">
      <p class="ocr-label">{{ $t('cards.extractedText') }}</p>
      <button class="btn-text" @click="copyOcr">{{ $t('cards.copy') }}</button>
    </div>
    <p class="ocr-content" :class="{ 'ocr-collapsed': !props.item.expandOcr }" v-html="highlighted"></p>
    <button
      v-if="props.item.ocr_text.length > 50"
      class="btn-text"
      @click="props.item.expandOcr = !props.item.expandOcr"
    >
      {{ props.item.expandOcr ? $t('cards.collapse') : $t('cards.expand') }}
      <span v-if="!props.item.expandOcr" class="remaining-hint">
        {{ $t('cards.remainingChars', { count: props.item.ocr_text.length - 50 }) }}
      </span>
    </button>
  </div>

  <!-- 尚无 OCR 文本时，提供即时识别入口 -->
  <div v-else class="ocr-run-panel">
    <button
      class="btn btn-sm btn-outline-primary"
      :disabled="props.item.ocrRunning"
      @click="runOcrForItem(props.item, ocrLanguages)"
    >
      {{ props.item.ocrRunning ? $t('cards.running') : $t('cards.runOcr') }}
    </button>
    <input
      v-model="props.item.ocrLangInput"
      type="text"
      :placeholder="$t('cards.ocrLangPlaceholder')"
      class="ocr-lang-input-sm"
    />
  </div>
</template>

<style scoped>
.ocr-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.ocr-header .ocr-label {
  margin-bottom: 0;
}
.remaining-hint {
  font-size: 11px;
  color: #6c8ee3;
  margin-left: 6px;
}
</style>
