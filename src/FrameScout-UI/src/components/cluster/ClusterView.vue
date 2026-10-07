<script setup lang="ts">
import type { ClusterGroup } from '@/types/search'
import { getAssetUrl } from '@/utils/media'

const props = defineProps<{
  clusters: ClusterGroup[]
  threshold: number
  /** 债单 B8：因「单帧簇 ≤50」限制而被丢弃的独特帧数 */
  truncatedSingleFrames: number
}>()

const emit = defineEmits<{
  (e: 'update:threshold', value: number): void
  (e: 'recluster'): void
  (e: 'open-preview', paths: string[], index: number): void
}>()

function onThresholdChange(e: Event) {
  emit('update:threshold', Number((e.target as HTMLInputElement).value))
}
</script>

<template>
  <div class="clustering-container">
    <div class="clustering-header">
      <h2>{{ $t('cluster.title') }}</h2>
      <div class="cluster-controls">
        <label>{{ $t('cluster.threshold') }}</label>
        <input
          type="range"
          :value="props.threshold"
          @change="onThresholdChange"
          min="0.65"
          max="0.95"
          step="0.05"
        />
        <span>{{ (props.threshold * 100).toFixed(0) }}%</span>
        <button class="btn btn-secondary btn-sm" @click="emit('recluster')">{{ $t('cluster.recluster') }}</button>
      </div>
    </div>

    <div v-if="props.clusters.length === 0" class="empty-clusters">
      <p>{{ $t('cluster.empty') }}</p>
      <p class="empty-hint">{{ $t('cluster.emptyHint') }}</p>
    </div>

    <div class="cluster-grid">
      <div v-for="group in props.clusters" :key="group.group_id" class="cluster-card">
        <div class="cluster-badge">
          {{ $t('cluster.group', { id: group.group_id, count: group.member_paths.length }) }}
        </div>
        <div class="cluster-thumbnails">
          <div
            v-for="(path, idx) in group.member_paths"
            :key="idx"
            class="cluster-thumb-wrap"
            :class="{ 'is-representative': path === group.representative_path }"
            :title="path === group.representative_path ? $t('cluster.representative') : $t('cluster.previewHint')"
            @click="emit('open-preview', group.member_paths, idx)"
          >
            <img :src="getAssetUrl(path)" class="cluster-thumb" />
            <span v-if="path === group.representative_path" class="representative-badge">★</span>
          </div>
        </div>
      </div>
    </div>

    <div v-if="props.truncatedSingleFrames > 0" class="truncate-hint">
      {{ $t('cluster.truncated', { count: props.truncatedSingleFrames }) }}
    </div>
  </div>
</template>

<style scoped>
.clustering-container {
  background: #12121a;
  padding: 25px;
  border-radius: 12px;
  border: 1px solid #8333ff;
}

.clustering-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid #222233;
  padding-bottom: 15px;
  margin-bottom: 20px;
}

.cluster-controls {
  display: flex;
  align-items: center;
  gap: 10px;
  color: #aaa;
}

.cluster-grid {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.cluster-card {
  background: #0a0a0e;
  padding: 15px;
  border-radius: 8px;
  border: 1px solid #222233;
}

.cluster-badge {
  font-weight: bold;
  background: var(--grad-amber-orange);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  margin-bottom: 12px;
}

.cluster-thumbnails {
  display: flex;
  gap: 10px;
  overflow-x: auto;
  padding-bottom: 5px;
}
.cluster-thumbnails::-webkit-scrollbar {
  height: 6px;
}
.cluster-thumbnails::-webkit-scrollbar-thumb {
  background: #333;
  border-radius: 3px;
}
.cluster-thumbnails::-webkit-scrollbar-track {
  background: transparent;
}

.cluster-thumb {
  width: 120px;
  height: 120px;
  object-fit: cover;
  border-radius: 6px;
  border: 1px solid #333;
}

.cluster-thumb-wrap {
  position: relative;
  flex-shrink: 0;
  cursor: pointer;
}
.cluster-thumb-wrap:hover .cluster-thumb {
  border-color: #6c8ee3;
}
.cluster-thumb-wrap.is-representative .cluster-thumb {
  border-color: #ffaa33;
  box-shadow: 0 0 0 2px rgba(255, 170, 51, 0.5);
}
.representative-badge {
  position: absolute;
  top: 4px;
  right: 4px;
  font-size: 13px;
  color: #ffaa33;
  text-shadow: 0 0 4px rgba(0, 0, 0, 0.8);
}

.empty-clusters {
  text-align: center;
  color: #888;
  padding: 30px 20px;
  font-size: 14px;
  background: rgba(255, 255, 255, 0.02);
  border-radius: 8px;
  border: 1px dashed #333;
}

.truncate-hint {
  margin-top: 16px;
  padding: 10px 14px;
  font-size: 12px;
  color: #ffaa33;
  background: rgba(255, 170, 51, 0.08);
  border: 1px dashed rgba(255, 170, 51, 0.3);
  border-radius: 8px;
}
.empty-hint {
  font-size: 12px;
  color: #666;
  margin-top: 8px;
}
</style>
