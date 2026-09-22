<script setup lang="ts">
import type { ClusterGroup } from '@/types/search'
import { getAssetUrl } from '@/utils/media'

const props = defineProps<{
  clusters: ClusterGroup[]
  threshold: number
}>()

const emit = defineEmits<{
  (e: 'update:threshold', value: number): void
  (e: 'recluster'): void
}>()

function onThresholdChange(e: Event) {
  emit('update:threshold', Number((e.target as HTMLInputElement).value))
}
</script>

<template>
  <div class="clustering-container">
    <div class="clustering-header">
      <h2>🧩 Visual Similarity Clusters</h2>
      <div class="cluster-controls">
        <label>Similarity Threshold:</label>
        <input
          type="range"
          :value="props.threshold"
          @change="onThresholdChange"
          min="0.65"
          max="0.95"
          step="0.05"
        />
        <span>{{ (props.threshold * 100).toFixed(0) }}%</span>
        <button class="btn btn-secondary btn-sm" @click="emit('recluster')">Re-cluster</button>
      </div>
    </div>

    <div v-if="props.clusters.length === 0" class="empty-clusters">
      <p>No visually similar groups found at current threshold.</p>
      <p class="empty-hint">Try lowering the similarity threshold to discover more groups.</p>
    </div>

    <div class="cluster-grid">
      <div v-for="group in props.clusters" :key="group.group_id" class="cluster-card">
        <div class="cluster-badge">
          Group #{{ group.group_id }} ({{ group.member_paths.length }} items)
        </div>
        <div class="cluster-thumbnails">
          <img
            v-for="(path, idx) in group.member_paths"
            :key="idx"
            :src="getAssetUrl(path)"
            class="cluster-thumb"
          />
        </div>
      </div>
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

.empty-clusters {
  text-align: center;
  color: #888;
  padding: 30px 20px;
  font-size: 14px;
  background: rgba(255, 255, 255, 0.02);
  border-radius: 8px;
  border: 1px dashed #333;
}
.empty-hint {
  font-size: 12px;
  color: #666;
  margin-top: 8px;
}
</style>
