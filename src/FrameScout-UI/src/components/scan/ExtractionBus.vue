<script setup lang="ts">
import { computed } from 'vue'
import type { ScanProgress } from '@/types/search'

const props = defineProps<{
  isScanning: boolean
  currentStatus: string
  currentFile: string
  scanProgress: ScanProgress
  scanMsg: string
  /** P0-8：已用时间（秒），useScanner 已算好，此前不传给 UI */
  scanElapsed: number
  /** P0-8：预计剩余时间（秒），按当前速度估算 */
  scanEta: number
}>()

// P1-9 / 债单 A6：停止扫描（用户终于有取消入口，不再只能等或杀进程）
const emit = defineEmits<{
  (e: 'stop'): void
}>()

const percent = computed(() => {
  const { current, total } = props.scanProgress
  return total > 0 ? (current / total) * 100 : 0
})

function formatDuration(seconds: number): string {
  if (!seconds || seconds < 0) return ''
  const m = Math.floor(seconds / 60)
  const s = Math.floor(seconds % 60)
  if (m > 0) return `${m}m ${s}s`
  return `${s}s`
}
</script>

<template>
  <div class="monitor-wrapper">
    <p v-if="props.scanMsg" class="scan-msg">{{ props.scanMsg }}</p>

    <div v-if="props.isScanning || props.currentFile" class="bus-panel">
      <div class="bus-header">
        <span class="bus-title">{{ $t('extraction.title') }}</span>
        <div class="bus-header-right">
          <span class="bus-status" :class="{ 'status-warning': props.currentStatus.includes('Batch') }">
            {{ props.currentStatus }}
          </span>
          <button v-if="props.isScanning" class="btn-stop" @click="emit('stop')">{{ $t('extraction.stop') }}</button>
        </div>
      </div>

      <div v-if="props.scanProgress.total > 0" class="progress-box">
        <div class="progress-info">
          <span>
            {{ $t('extraction.progress', { current: props.scanProgress.current, total: props.scanProgress.total }) }}
          </span>
          <span class="progress-percentage">{{ percent.toFixed(1) }}%</span>
          <span v-if="props.scanElapsed > 0" class="progress-eta">
            {{ $t('extraction.eta', { elapsed: formatDuration(props.scanElapsed), eta: props.scanEta > 0 ? formatDuration(props.scanEta) : '…' }) }}
          </span>
        </div>
        <div class="progress-track">
          <div class="progress-fill" :style="{ width: percent + '%' }"></div>
        </div>
      </div>

      <div class="current-file-text">&gt; {{ props.currentFile || $t('extraction.awaiting') }}</div>
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
  align-items: center;
  border-bottom: 1px solid #1a1a24;
  padding-bottom: 8px;
}

.bus-header-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.btn-stop {
  padding: 4px 14px;
  background: rgba(248, 113, 113, 0.12);
  border: 1px solid rgba(248, 113, 113, 0.35);
  border-radius: 6px;
  color: #f87171;
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s;
  font-family: monospace;
}

.btn-stop:hover {
  background: rgba(248, 113, 113, 0.22);
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
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.progress-percentage {
  color: #2fe9e9;
  font-weight: bold;
}
.progress-eta {
  color: #ffaa00;
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
