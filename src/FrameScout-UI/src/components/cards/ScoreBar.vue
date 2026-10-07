<script setup lang="ts">
/**
 * ScoreBar — 得分来源条（P2-6 / 第三轮「相似度无解释」）。
 *
 * 消费 search_unified 返回的 `matches: [{channel, score}]`，画一条横向堆叠条，
 * 让「为什么这条结果排第一」一眼可读：画面 / OCR / 笔记 / 文件名各占多少。
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

// P1-11：修占比失真。此前非视觉通道硬编码 100（=「100% 完美匹配」），
// 而字面命中本质是二值的（命中 = score 1.0），「命中」≠「完美匹配」。
// 现统一到「人类可读贡献」量纲：视觉走原始余弦映射，字面命中映射为
// 单通道字面命中的合理贡献（65，与 resolveLiteralScore 的口径一致），
// 使视觉与字面可比、不再被「100 满分」误导。
const LITERAL_HIT_WEIGHT = 65

function channelWeight(m: ChannelMatch): number {
  if (m.channel === 'visual') return mapToHumanScore(m.score, false)
  return LITERAL_HIT_WEIGHT
}

const total = computed(() => {
  const sum = props.matches.reduce((s, m) => s + channelWeight(m), 0)
  return sum > 0 ? sum : 1
})

const segments = computed(() =>
  props.matches.map((m) => {
    const meta = CHANNEL_META[m.channel]
    return {
      label: meta ? t(meta.labelKey) : m.channel,
      color: meta?.color ?? '#8888a0',
      pct: (channelWeight(m) / total.value) * 100,
    }
  })
)
</script>

<template>
  <div v-if="props.matches.length" class="score-bar-wrap">
    <div class="score-bar">
      <div
        v-for="(seg, i) in segments"
        :key="i"
        class="score-seg"
        :style="{ width: seg.pct + '%', background: seg.color }"
      ></div>
    </div>
    <div class="score-legend">
      <span v-for="(seg, i) in segments" :key="i" class="score-legend-item">
        <i class="score-dot" :style="{ background: seg.color }"></i>
        {{ seg.label }} {{ Math.round(seg.pct) }}%
      </span>
    </div>
  </div>
</template>

<style scoped>
.score-bar-wrap {
  display: flex;
  flex-direction: column;
  gap: 5px;
  margin: 6px 0;
}

.score-bar {
  display: flex;
  height: 6px;
  width: 100%;
  border-radius: 3px;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.06);
}

.score-seg {
  height: 100%;
  transition: width 0.3s ease;
}

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
</style>
