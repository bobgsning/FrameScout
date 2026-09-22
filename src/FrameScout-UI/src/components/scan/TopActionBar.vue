<script setup lang="ts">
import { useScanner } from '@/composables/useScanner'

const props = defineProps<{
  folderPath: string
  scanMode: string
  enableOcr: boolean
  ocrLanguages: string
  isScanning: boolean
  isClusteringView: boolean
}>()

const emit = defineEmits<{
  (e: 'update:folderPath', value: string): void
  (e: 'update:scanMode', value: string): void
  (e: 'update:enableOcr', value: boolean): void
  (e: 'update:ocrLanguages', value: string): void
  (e: 'start-scan'): void
  (e: 'index-files'): void
  (e: 'toggle-clustering'): void
  (e: 'clean-ghosts'): void
}>()

// 目录选择器属于「扫描」域的无状态动作，直接从单例 composable 取
const { selectFolder } = useScanner()
</script>

<template>
  <div class="top-action-bar">
    <div class="bar-row">
      <input
        :value="props.folderPath"
        @input="emit('update:folderPath', ($event.target as HTMLInputElement).value)"
        id="folderPath"
        type="text"
        placeholder="Paste folder path here, or click Browse..."
        class="custom-input path-input"
      />
      <button class="btn btn-secondary" @click="selectFolder">📂 Browse</button>
    </div>

    <div class="bar-row">
      <select
        :value="props.scanMode"
        @change="emit('update:scanMode', ($event.target as HTMLSelectElement).value)"
        id="scanMode"
        class="custom-select"
      >
        <option value="all">All Media</option>
        <option value="image">Image Only</option>
        <option value="video">Video Only</option>
      </select>

      <div class="ocr-controls">
        <label class="ocr-toggle">
          <input
            type="checkbox"
            :checked="props.enableOcr"
            @change="emit('update:enableOcr', ($event.target as HTMLInputElement).checked)"
          />
          🔠 OCR
        </label>
        <input
          v-if="props.enableOcr"
          :value="props.ocrLanguages"
          @input="emit('update:ocrLanguages', ($event.target as HTMLInputElement).value)"
          type="text"
          placeholder="e.g. en, ch_sim, ja"
          class="custom-input ocr-lang-input"
        />
      </div>

      <button
        class="btn btn-primary"
        :disabled="props.isScanning || !props.folderPath"
        @click="emit('start-scan')"
      >
        {{ props.isScanning ? 'Indexing Data...' : 'Start Indexing' }}
      </button>

      <button
        class="btn btn-secondary"
        title="Select specific files to index"
        @click="emit('index-files')"
      >
        📤 Index Files
      </button>

      <button
        class="btn btn-cluster"
        title="Group visually similar images"
        @click="emit('toggle-clustering')"
      >
        {{ props.isClusteringView ? '🖼️ Standard View' : '🧩 Visual Clusters' }}
      </button>

      <button
        class="btn btn-danger"
        title="Remove physically deleted records"
        @click="emit('clean-ghosts')"
      >
        🧹 Purge Ghosts
      </button>
    </div>
  </div>
</template>

<style scoped>
.top-action-bar {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 12px;
}

.path-input {
  min-width: 280px;
  max-width: 520px;
  flex: 1 1 auto;
  padding: 10px 15px;
  border-radius: 6px;
  border: 1px solid #333344;
  background: #121218;
  color: #fff;
  outline: none;
}

.ocr-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}
.ocr-toggle {
  cursor: pointer;
  font-size: 14px;
  white-space: nowrap;
}
.ocr-lang-input {
  width: 140px;
  padding: 4px 8px;
  font-size: 13px;
}

.bar-row {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  align-items: center;
  justify-content: center;
}

@media (max-width: 640px) {
  .bar-row {
    flex-direction: column;
    align-items: stretch;
  }
  .bar-row > * {
    width: 100%;
    text-align: center;
  }
}
</style>
