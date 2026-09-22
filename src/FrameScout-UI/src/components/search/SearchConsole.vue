<script setup lang="ts">
import { getAssetUrl } from '@/utils/media'

const props = defineProps<{
  query: string
  pageSize: number
  sFilename: boolean
  sNote: boolean
  sOcr: boolean
  sVector: boolean
  isImageSearch: boolean
  searchImagePath: string
  searchMsg: string
}>()

const emit = defineEmits<{
  (e: 'update:query', value: string): void
  (e: 'update:pageSize', value: number): void
  (e: 'update:sFilename', value: boolean): void
  (e: 'update:sNote', value: boolean): void
  (e: 'update:sOcr', value: boolean): void
  (e: 'update:sVector', value: boolean): void
  (e: 'search'): void
  (e: 'search-by-image'): void
  (e: 'clear-image-search'): void
  (e: 'save-smart-folder'): void
  (e: 'show-all'): void
}>()

function onCheckbox(key: 'sFilename' | 'sNote' | 'sOcr' | 'sVector', e: Event) {
  const checked = (e.target as HTMLInputElement).checked
  // 勾选框变化即触发重新搜索
  emit(`update:${key}` as any, checked)
  emit('search')
}

function onPageSizeChange(e: Event) {
  const value = Number((e.target as HTMLSelectElement).value)
  emit('update:pageSize', value)
  emit('search')
}
</script>

<template>
  <div class="search-console">
    <div class="search-options">
      <label>
        <input
          type="checkbox"
          :checked="props.sFilename"
          @change="onCheckbox('sFilename', $event)"
        />
        📁 Filename
      </label>
      <label>
        <input type="checkbox" :checked="props.sNote" @change="onCheckbox('sNote', $event)" />
        📝 Note
      </label>
      <label>
        <input type="checkbox" :checked="props.sOcr" @change="onCheckbox('sOcr', $event)" />
        🔍 OCR
      </label>
      <label>
        <input type="checkbox" :checked="props.sVector" @change="onCheckbox('sVector', $event)" />
        💡 Semantic AI
      </label>

      <div class="option-divider"></div>

      <label>
        Items/Page:
        <select :value="props.pageSize" @change="onPageSizeChange" class="custom-select-sm">
          <option :value="4">4</option>
          <option :value="8">8</option>
          <option :value="16">16</option>
          <option :value="50">50</option>
          <option :value="150">150</option>
        </select>
      </label>
    </div>

    <!-- 以图搜图的目标图预览 -->
    <div v-if="props.isImageSearch && props.searchImagePath" class="visual-target-banner">
      <img :src="getAssetUrl(props.searchImagePath)" class="target-img" />
      <div class="target-info">
        <p class="target-title">🎯 Visual Target</p>
        <p class="target-path">{{ props.searchImagePath }}</p>
      </div>
      <button class="btn btn-danger btn-sm" @click="emit('clear-image-search')">✖ Cancel</button>
    </div>

    <div class="search-bar">
      <input
        :value="props.query"
        @input="emit('update:query', ($event.target as HTMLInputElement).value)"
        @keyup.enter="emit('search')"
        id="searchQuery"
        type="text"
        placeholder="Multi-dim search: description, filename, or notes..."
        class="custom-input search-input"
      />
      <button class="btn btn-primary search-btn" @click="emit('search')">Search</button>

      <button
        class="btn btn-outline-primary"
        title="Select an image to find similar frames!"
        @click="emit('search-by-image')"
      >
        🖼️ Similar
      </button>

      <button
        v-if="props.query"
        class="btn btn-secondary"
        title="Save this search as a Smart Folder"
        @click="emit('save-smart-folder')"
      >
        ⭐ Save Smart
      </button>

      <button
        class="btn btn-outline-primary"
        title="Display all indexed files"
        @click="emit('show-all')"
      >
        📋 All Files
      </button>
    </div>

    <p v-if="props.searchMsg" class="error-msg">{{ props.searchMsg }}</p>
  </div>
</template>

<style scoped>
.search-console {
  background: #12121a;
  padding: 25px;
  border-radius: 12px;
  max-width: 800px;
  margin: 0 auto 30px auto;
  border: 1px solid #222233;
}

.search-options {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 15px;
  color: #aaa;
  font-size: 13px;
  margin-bottom: 15px;
}

.option-divider {
  width: 1px;
  height: 16px;
  background: #333;
}

.search-bar {
  display: flex;
  justify-content: center;
  gap: 10px;
}

.search-input {
  width: 100%;
  max-width: 480px;
  font-size: 16px;
  border-color: #8333ff;
}

/* 以图搜图目标图横幅（原实现引用了 class 但未定义样式，此处补齐） */
.visual-target-banner {
  display: flex;
  align-items: center;
  gap: 12px;
  background: #0c0c12;
  border: 1px solid #2a2a3a;
  border-radius: 8px;
  padding: 10px 14px;
  margin-bottom: 15px;
}

.target-img {
  width: 64px;
  height: 64px;
  object-fit: cover;
  border-radius: 6px;
  border: 1px solid #333;
  flex-shrink: 0;
}

.target-info {
  flex: 1;
  min-width: 0;
}

.target-title {
  margin: 0 0 4px 0;
  font-size: 13px;
  font-weight: bold;
  color: #67e5e5;
}

.target-path {
  margin: 0;
  font-size: 11px;
  color: #888;
  word-break: break-all;
}
</style>
