<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { TextEntryHit } from '@/types/search'
import { formatScore } from '@/utils/score'
import { useToast } from '@/composables/useToast'

const props = defineProps<{
  item: TextEntryHit
}>()

const { push } = useToast()
const { t } = useI18n()

// BGE-M3 dense 余弦 → 人类可读分（复用 score.ts 的 bge 量纲映射）
const scoreText = computed(() => formatScore(props.item.similarity, false, 'bge'))

/** 超过该长度才提供展开按钮——短文本不必多一个按钮（与 OcrPanel 的 50 字同理） */
const EXPAND_LIMIT = 120
const canExpand = computed(() => props.item.content.length > EXPAND_LIMIT)

// P1-6：复制纯文本内容
async function copyContent() {
  try {
    await navigator.clipboard.writeText(props.item.content)
    push(t('cards.copied'), 'success')
  } catch {
    push(t('cards.copyFailed'), 'error')
  }
}
</script>

<template>
  <div class="text-entry-card">
    <p class="text-content" :class="{ 'text-collapsed': !props.item.expand }">
      {{ props.item.content }}
    </p>

    <button
      v-if="canExpand"
      class="btn-text"
      @click="props.item.expand = !props.item.expand"
    >
      {{ props.item.expand ? $t('cards.collapse') : $t('cards.expand') }}
      <span v-if="!props.item.expand" class="remaining-hint">
        {{ $t('cards.remainingChars', { count: props.item.content.length - EXPAND_LIMIT }) }}
      </span>
    </button>

    <div class="text-meta-row">
      <span class="text-source">{{ props.item.source_uri || $t('cards.noSource') }}</span>
      <button class="btn-text" @click="copyContent">{{ $t('cards.copy') }}</button>
      <span class="text-score">{{ scoreText }}</span>
    </div>
  </div>
</template>

<style scoped>
.text-entry-card {
  background: #14141e;
  padding: 15px;
  border-radius: 10px;
  width: 340px;
  border: 1px solid #222233;
  display: flex;
  flex-direction: column;
  gap: 8px;
  transition: transform 0.2s;
}
.text-entry-card:hover {
  transform: translateY(-3px);
  border-color: #8333ff;
}
.text-content {
  font-size: 13px;
  color: #ddd;
  line-height: 1.55;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
}
/* 折叠态：只露出开头几行，点 Expand 才看全文 */
.text-collapsed {
  max-height: 110px;
  overflow: hidden;
}
.text-meta-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  margin-top: auto;
}
.text-source {
  font-size: 11px;
  color: #888;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}
.remaining-hint {
  font-size: 11px;
  color: #6c8ee3;
  margin-left: 6px;
}
.text-score {
  font-size: 13px;
  font-weight: bold;
  background: var(--grad-yellow-orange);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  flex-shrink: 0;
}
</style>
