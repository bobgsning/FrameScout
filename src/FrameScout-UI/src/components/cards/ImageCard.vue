<script setup lang="ts">
import { computed } from 'vue'
import type { SearchResult } from '@/types/search'
import { getAssetUrl } from '@/utils/media'
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

// P0-5：单击图片直接开大图（此前只能右键菜单找「大图查看」）
const emit = defineEmits<{
  openLightbox: [item: SearchResult]
}>()

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
const visibleTags = computed(() =>
  (props.item.matched_tags || []).filter((t) => !t.includes('Semantic'))
)
</script>

<template>
  <img
    :src="getAssetUrl(props.item.path)"
    :alt="props.item.path"
    loading="lazy"
    @error="props.item.isMissing = true"
    @click="emit('openLightbox', props.item)"
    class="media-preview clickable"
  />

  <!-- 低置信度折叠条：点击展开 -->
  <div
    v-if="props.item.collapsedLowScore"
    class="low-score-collapsed-bar"
    @click="props.item.collapsedLowScore = false"
  >
    <span class="collapse-icon">❌</span>
    <span class="collapse-text">{{ $t('cards.lowConfidence', { score: scoreText }) }}</span>
    <span class="expand-arrow">▶</span>
  </div>

  <div v-show="!props.item.collapsedLowScore" class="collapsible-content">
    <p class="file-path" v-html="highlightedPath"></p>

    <div class="card-meta-row">
      <p class="type-text">{{ $t('cards.staticImage') }}</p>
      <p class="match-score">{{ scoreText }}</p>
    </div>

    <div class="tags-wrapper">
      <span v-for="tag in visibleTags" :key="tag" class="tag-badge">{{ tag }}</span>
      <!-- 语义徽章/低分徽章基于“剔除 Semantic 后的可见标签数”判断：
           纯向量（语义）匹配时 visibleTags 为空，应在此显示 💡 Semantic；
           否则若既无其它标签又低分，显示 👻 Low Confidence。 -->
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

    <!-- 仅低置信度卡片才有收起按钮 -->
    <div v-if="props.item.collapsedLowScore !== undefined" class="collapse-footer">
      <button class="btn-collapse-up" @click="props.item.collapsedLowScore = true">{{ $t('cards.collapse') }}</button>
    </div>
  </div>
</template>

<style scoped>
/* P0-5：图片可点击预览 */
.media-preview.clickable {
  cursor: zoom-in;
}
.type-text {
  font-size: 12px;
  color: #888;
  margin: 0;
}

/* Low Score Collapsible Bar */
.low-score-collapsed-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: linear-gradient(135deg, rgba(255, 68, 68, 0.08), rgba(255, 68, 68, 0.04));
  border: 1px solid rgba(255, 68, 68, 0.25);
  border-radius: 8px;
  padding: 10px 14px;
  cursor: pointer;
  font-size: 13px;
  color: #ff6677;
  transition: background 0.2s, border-color 0.2s, box-shadow 0.2s;
  margin: 6px 0 4px 0;
  box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.2);
}
.low-score-collapsed-bar:hover {
  background: linear-gradient(135deg, rgba(255, 68, 68, 0.14), rgba(255, 68, 68, 0.06));
  border-color: rgba(255, 68, 68, 0.4);
  box-shadow: inset 0 1px 4px rgba(255, 68, 68, 0.1);
}

.collapse-icon {
  margin-right: 10px;
  font-size: 16px;
  flex-shrink: 0;
}
.collapse-text {
  flex: 1;
  margin-right: 8px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.expand-arrow {
  font-size: 14px;
  opacity: 0.6;
  transition: transform 0.2s, opacity 0.2s;
}
.low-score-collapsed-bar:hover .expand-arrow {
  transform: translateX(3px);
  opacity: 1;
}

.collapsible-content {
  margin-top: 6px;
}

.collapse-footer {
  display: flex;
  justify-content: flex-end;
  margin-top: 8px;
}

.btn-collapse-up {
  background: none;
  border: none;
  color: #888;
  font-size: 12px;
  cursor: pointer;
  padding: 4px 10px;
  border-radius: 4px;
  transition: background 0.2s, color 0.2s;
}
.btn-collapse-up:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #bbb;
}
</style>
