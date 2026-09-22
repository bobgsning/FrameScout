<script setup lang="ts">
import { computed } from 'vue'
import type { ScanProgress } from '@/types/search'

const props = defineProps<{
  isScanning: boolean
  currentStatus: string
  currentFile: string
  scanProgress: ScanProgress
  scanMsg: string
}>()

const percent = computed(() => {
  const { current, total } = props.scanProgress
  return total > 0 ? (current / total) * 100 : 0
})
</script>

<template>
  <div class="monitor-wrapper">
    <p v-if="props.scanMsg" class="scan-msg">{{ props.scanMsg }}</p>

    <div v-if="props.isScanning || props.currentFile" class="bus-panel">
      <div class="bus-header">
        <span class="bus-title">⚡ Real-time Extraction Bus</span>
        <span class="bus-status" :class="{ 'status-warning': props.currentStatus.includes('Batch') }">
          {{ props.currentStatus }}
        </span>
      </div>

      <div v-if="props.scanProgress.total > 0" class="progress-box">
        <div class="progress-info">
          <span>
            Parsing Progress: {{ props.scanProgress.current }} / {{ props.scanProgress.total }},
          </span>
          <span class="progress-percentage">{{ percent.toFixed(1) }}%</span>
        </div>
        <div class="progress-track">
          <div class="progress-fill" :style="{ width: percent + '%' }"></div>
        </div>
      </div>

      <div class="current-file-text">&gt; {{ props.currentFile || 'Awaiting signal...' }}</div>
    </div>
  </div>
</template>

<style scoped>
.monitor-wrapper {
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  margin-bottom: 25px;
}

.scan-msg {
  color: #ebff1a;
  font-weight: bold;
  margin-bottom: 10px;
}

.bus-panel {
  width: 100%;
  max-width: 700px;
  background: #0c0c10;
  padding: 15px;
  border-radius: 8px;
  border: 1px solid #222233;
  text-align: left;
  font-family: monospace;
}

.bus-header {
  display: flex;
  justify-content: space-between;
  border-bottom: 1px solid #1a1a24;
  padding-bottom: 8px;
}

.bus-title {
  background: var(--grad-purple-blue);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  font-weight: bold;
}

.bus-status {
  color: #2fe9e9;
}
.status-warning {
  color: #ffaa00;
}

.progress-info {
  font-size: 12px;
  color: #aaa;
  margin-top: 8px;
}
.progress-percentage {
  color: #2fe9e9;
  font-weight: bold;
}

.progress-track {
  width: 100%;
  background: #1a1a24;
  border-radius: 4px;
  height: 8px;
  overflow: hidden;
  margin-top: 5px;
}
.progress-fill {
  background: var(--grad-blue-cyan);
  height: 100%;
  transition: width 0.2s ease;
}

.current-file-text {
  color: #777;
  font-size: 12px;
  margin-top: 8px;
  word-break: break-all;
}
</style>
