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
  (e: 'review-changes'): void
  (e: 'show-reports'): void
  (e: 'manage-text'): void
  (e: 'show-all'): void
  (e: 'insert-text'): void
  (e: 'open-settings'): void
  (e: 'open-timeline'): void
}>()

// 目录选择器属于「扫描」域的无状态动作，直接从单例 composable 取
const { selectFolder } = useScanner()
</script>

<template>
  <div class="top-action-bar">
    <!-- 采集组：把数据装进库（只做「进」） -->
    <section class="bar-group">
      <span class="group-label">{{ $t('scan.capture') }}</span>

      <div class="bar-row">
        <input
          :value="props.folderPath"
          @input="emit('update:folderPath', ($event.target as HTMLInputElement).value)"
          id="folderPath"
          type="text"
          :placeholder="$t('scan.folderPlaceholder')"
          class="custom-input path-input"
        />
        <button class="btn btn-secondary" @click="selectFolder">{{ $t('scan.browse') }}</button>
      </div>

      <!-- 配置行：扫描范围 + OCR（先定位 → 再配置） -->
      <div class="bar-row">
        <select
          :value="props.scanMode"
          @change="emit('update:scanMode', ($event.target as HTMLSelectElement).value)"
          id="scanMode"
          class="custom-select"
        >
          <option value="all">{{ $t('scan.allMedia') }}</option>
          <option value="image">{{ $t('scan.imageOnly') }}</option>
          <option value="video">{{ $t('scan.videoOnly') }}</option>
        </select>

        <div class="ocr-controls">
          <label class="ocr-toggle">
            <input
              type="checkbox"
              :checked="props.enableOcr"
              @change="emit('update:enableOcr', ($event.target as HTMLInputElement).checked)"
            />
            {{ $t('scan.ocr') }}
          </label>
          <input
            v-if="props.enableOcr"
            :value="props.ocrLanguages"
            @input="emit('update:ocrLanguages', ($event.target as HTMLInputElement).value)"
            type="text"
            :placeholder="$t('scan.ocrPlaceholder')"
            class="custom-input ocr-lang-input"
          />
        </div>
      </div>

      <!-- 执行行：主按钮「开始索引」+ 次按钮「索引文件」 -->
      <div class="bar-row action-row">
        <button
          class="btn btn-primary btn-scan-start"
          :disabled="props.isScanning || !props.folderPath"
          :title="!props.folderPath ? $t('scan.needFolderHint') : ''"
          @click="emit('start-scan')"
        >
          {{ props.isScanning ? $t('scan.indexing') : $t('scan.startIndexing') }}
        </button>

        <button
          class="btn btn-secondary"
          :title="$t('scan.selectFilesHint')"
          @click="emit('index-files')"
        >
          {{ $t('scan.indexFiles') }}
        </button>
      </div>
    </section>

    <!-- 探索与维护组：审视库里已经有什么（只做「看」与「审」，动作由用户触发） -->
    <section class="bar-group">
      <span class="group-label">{{ $t('scan.explore') }}</span>

      <div class="bar-row">
        <button
          class="btn btn-cluster"
          :title="$t('scan.clusterHint')"
          @click="emit('toggle-clustering')"
        >
          {{ props.isClusteringView ? $t('scan.standardView') : $t('scan.visualClusters') }}
        </button>

        <button
          class="btn btn-secondary"
          :title="$t('scan.activityHint')"
          @click="emit('show-reports')"
        >
          {{ $t('scan.activity') }}
        </button>

        <button
          class="btn btn-secondary"
          :title="$t('scan.reviewChangesHint')"
          @click="emit('review-changes')"
        >
          {{ $t('scan.reviewChanges') }}
        </button>

        <button
          class="btn btn-secondary"
          :title="$t('scan.reviewGhostsHint')"
          @click="emit('clean-ghosts')"
        >
          {{ $t('scan.reviewGhosts') }}
        </button>

        <button
          class="btn btn-secondary"
          :title="$t('scan.textLibraryHint')"
          @click="emit('manage-text')"
        >
          {{ $t('scan.textLibrary') }}
        </button>

        <button
          class="btn btn-secondary"
          :title="$t('scan.addTextHint')"
          @click="emit('insert-text')"
        >
          {{ $t('scan.addText') }}
        </button>

        <button
          class="btn btn-secondary"
          :title="$t('scan.showAllHint')"
          @click="emit('show-all')"
        >
          {{ $t('scan.showAll') }}
        </button>

        <button
          class="btn btn-secondary"
          :title="$t('scan.timelineHint')"
          @click="emit('open-timeline')"
        >
          {{ $t('scan.timeline') }}
        </button>

        <button
          class="btn btn-settings"
          :title="$t('scan.settingsHint')"
          @click="emit('open-settings')"
        >
          {{ $t('scan.settings') }}
        </button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.top-action-bar {
  display: flex;
  flex-direction: column;
  gap: 14px;
  margin-bottom: 16px;
}

/* 分组容器：弱边框卡片，把「采集」与「探索维护」在视觉上隔开 */
.bar-group {
  display: flex;
  flex-direction: column;
  gap: 10px;
  background: #12121a;
  border: 1px solid #1e1e2a;
  border-radius: 10px;
  padding: 12px 14px;
}

/* 分组小标题：置于容器顶部，说明这一组的职责 */
.group-label {
  font-size: 12px;
  font-weight: bold;
  color: #67e5e5;
  letter-spacing: 0.5px;
  text-transform: uppercase;
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
  gap: 10px;
  align-items: center;
  justify-content: center;
}

/* 执行行：主按钮「开始索引」更宽更醒目 */
.action-row {
  padding-top: 2px;
}
.btn-scan-start {
  min-width: 160px;
}

/* 设置按钮：醒目填充色（Batch 31） */
.btn-settings {
  background: linear-gradient(135deg, #8333ff, #6c8ee3);
  border: none;
  color: #fff;
  font-weight: 600;
  padding: 8px 18px;
  border-radius: 6px;
  cursor: pointer;
  box-shadow: 0 0 10px rgba(131, 51, 255, 0.35);
  transition: filter 0.15s, transform 0.1s;
}
.btn-settings:hover {
  filter: brightness(1.12);
  transform: translateY(-1px);
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
