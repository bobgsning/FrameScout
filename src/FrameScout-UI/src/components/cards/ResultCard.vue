<script setup lang="ts">
import { computed } from 'vue'
import type { SearchResult } from '@/types/search'
import { isVideo } from '@/utils/media'
import { clearVideoTimer } from '@/utils/videoTimers'
import VideoCard from './VideoCard.vue'
import ImageCard from './ImageCard.vue'

const props = defineProps<{
  item: SearchResult
  searchQuery: string
  isImageSearch: boolean
}>()

const isVideoItem = computed(() => isVideo(props.item.path))

function handleVideoError() {
  clearVideoTimer(props.item.path)
  props.item.isMissing = true
}

function handleVideoLoaded() {
  clearVideoTimer(props.item.path)
}
</script>

<template>
  <div class="result-card">
    <!-- 文件不可达（磁盘已删除 / 视频 25s 未加载出） -->
    <div v-if="props.item.isMissing" class="missing-card">
      <span class="missing-icon">👻</span>
      <span>File Unavailable</span>
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
    />
  </div>
</template>
