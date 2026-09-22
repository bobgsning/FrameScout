<script setup lang="ts">
import { computed } from 'vue'
import type { SearchResult } from '@/types/search'
import { getAssetUrl, withFrameAnchor } from '@/utils/media'
import { formatScore, getNumericScore } from '@/utils/score'
import { highlight } from '@/utils/highlight'
import OcrPanel from './OcrPanel.vue'
import NotePanel from './NotePanel.vue'

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
const scoreText = computed(() => formatScore(props.item.score, props.isImageSearch))
const numericScore = computed(() => getNumericScore(props.item.score, props.isImageSearch))
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
      <p class="badge-timestamp">⏱️ Sec {{ props.item.timestamp }}</p>
      <p class="match-score">{{ scoreText }}</p>
    </div>

    <div class="tags-wrapper">
      <span v-for="tag in visibleTags" :key="tag" class="tag-badge">{{ tag }}</span>
      <!-- 语义徽章/低分徽章基于“剔除 Semantic 后的可见标签数”判断（见 ImageCard 同逻辑） -->
      <span
        v-if="numericScore >= 60.0 && visibleTags.length === 0"
        class="tag-badge semantic-tag"
      >
        💡 Semantic
      </span>
      <span
        v-if="numericScore < 60.0 && visibleTags.length === 0"
        class="tag-badge low-tag"
      >
        👻 Low Confidence
      </span>
    </div>

    <OcrPanel :item="props.item" :search-query="props.searchQuery" :is-image-search="props.isImageSearch" />
    <NotePanel :item="props.item" />
  </div>
</template>
