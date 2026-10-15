<script setup lang="ts">
/**
 * ScoreBar — 得分来源（P2-6 / 第三轮「相似度无解释」）。
 *
 * 第七轮修复：字面命中（OCR/笔记/文件名）是二值命中，后端没有可比较的通道分，
 * 因此**不再画「堆叠占比条」**（那需要虚构权重，制造「画面 58%」这种假解释）。
 * 改为：视觉通道显示真实「匹配度」，字面通道只标「✓ 命中」。
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ChannelMatch } from '@/types/search'
import { mapToHumanScore } from '@/utils/score'

const props = defineProps<{
  matches: ChannelMatch[]
}>()

const { t } = useI18n()

const CHANNEL_META: Record<string, { labelKey: string; color: string }> = {
  visual: { labelKey: 'cards.frame', color: '#6c8ee3' },
  ocr: { labelKey: 'cards.ocrScore', color: '#4ade80' },
  note: { labelKey: 'cards.noteScore', color: '#ffaa33' },
  filename: { labelKey: 'cards.filenameScore', color: '#b06cff' },
}

const visualMatch = computed(() => props.matches.find((m) => m.channel === 'visual'))
const literalMatches = computed(() => props.matches.filter((m) => m.channel !== 'visual'))

/** 视觉匹配度（人类可读分），只在有视觉命中时显示 */
const visualScore = computed(() =>
  visualMatch.value ? Math.round(mapToHumanScore(visualMatch.value.score, false)) : null
)

function metaOf(channel: string) {
  const m = CHANNEL_META[channel]
  return m ? { label: t(m.labelKey), color: m.color } : { label: channel, color: '#8888a0' }
}
</script>

<template>
  <div v-if="props.matches.length" class="score-bar-wrap">
    <div class="score-legend">
      <span v-if="visualMatch && visualScore != null" class="score-legend-item">
        <i class="score-dot" :style="{ background: CHANNEL_META.visual.color }"></i>
        {{ $t('cards.frame') }} {{ visualScore }}%
      </span>
      <span v-for="m in literalMatches" :key="m.channel" class="score-legend-item">
        <i class="score-dot" :style="{ background: metaOf(m.channel).color }"></i>
        {{ metaOf(m.channel).label }} ✓
      </span>
    </div>
    <p v-if="literalMatches.length" class="score-literal-note">{{ t('cards.literalHitNote') }}</p>
  </div>
</template>

<style scoped>
.score-bar-wrap {
  display: flex;
  flex-direction: column;
  gap: 5px;
  margin: 6px 0;
}

/* 第八轮：`.score-bar` / `.score-seg`（堆叠占比条）已随「去虚构权重」一起删除，
   此处不再保留死样式。 */

.score-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.score-legend-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: #8888a0;
}

.score-dot {
  width: 8px;
  height: 8px;
  border-radius: 2px;
  display: inline-block;
}

.score-literal-note {
  margin: 2px 0 0;
  font-size: 10px;
  color: #666677;
  line-height: 1.4;
}
</style>
