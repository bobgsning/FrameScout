<script setup lang="ts">
import { computed } from 'vue'
import type { SearchResult } from '@/types/search'
import { getAssetUrl, withFrameAnchor } from '@/utils/media'
import { formatScore, getNumericScore, formatSearchScore, getSearchNumericScore } from '@/utils/score'
import { highlight } from '@/utils/highlight'
import OcrPanel from './OcrPanel.vue'
import NotePanel from './NotePanel.vue'
import ScoreBar from './ScoreBar.vue'

const props = defineProps<{
  item: SearchResult
  searchQuery: string
  isImageSearch: boolean
}>()

const emit = defineEmits<{
  (e: 'video-error'): void
  (e: 'video-loaded'): void
}>()

/** 视频定位到命中的具体帧 */
const src = computed(() => withFrameAnchor(getAssetUrl(props.item.path), props.item.timestamp))
// 方案 C：文字搜索走结构化自决映射；以图搜图 / BGE 保持原映射。
const scoreText = computed(() => {
  if (props.item.scoreKind === 'bge') return formatScore(props.item.score, false, 'bge')
  if (props.isImageSearch) return formatScore(props.item.score, true)
  return formatSearchScore(props.item)
})
const numericScore = computed(() => {
  if (props.item.scoreKind === 'bge') return getNumericScore(props.item.score, false, 'bge')
  if (props.isImageSearch) return getNumericScore(props.item.score, true)
  return getSearchNumericScore(props.item)
})
const highlightedPath = computed(() =>
  highlight(props.item.path, props.searchQuery, props.isImageSearch)
)

/** 过滤掉语义标记标签，改由分数徽章统一表达 */
const visibleTags = computed(() =>
  (props.item.matched_tags || []).filter((t) => !t.includes('Semantic'))
)
</script>

<template>
  <video
    :src="src"
    @error="emit('video-error')"
    @loadeddata="emit('video-loaded')"
    controls
    preload="metadata"
    class="media-preview"
  ></video>

  <div class="card-content">
    <p class="file-path" v-html="highlightedPath"></p>

    <div class="card-meta-row">
      <p class="badge-timestamp">{{ $t('cards.sec', { time: props.item.timestamp }) }}</p>
      <p class="match-score">{{ scoreText }}</p>
    </div>

    <div class="tags-wrapper">
      <span v-for="tag in visibleTags" :key="tag" class="tag-badge">{{ tag }}</span>
      <!-- 语义徽章/低分徽章基于“剔除 Semantic 后的可见标签数”判断（见 ImageCard 同逻辑） -->
      <span
        v-if="numericScore >= 60.0 && visibleTags.length === 0"
        class="tag-badge semantic-tag"
      >
        {{ $t('cards.semantic') }}
      </span>
      <span
        v-if="numericScore < 60.0 && visibleTags.length === 0"
        class="tag-badge low-tag"
      >
        {{ $t('cards.lowConfidenceBadge') }}
      </span>
    </div>

    <!-- 得分来源条（P2-6）：为什么它排第一 -->
    <ScoreBar v-if="props.item.matches?.length" :matches="props.item.matches" />

    <OcrPanel :item="props.item" :search-query="props.searchQuery" :is-image-search="props.isImageSearch" />
    <NotePanel :item="props.item" />
  </div>
</template>
