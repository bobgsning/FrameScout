<script setup lang="ts">
import type { SmartFolder } from '@/types/search'

defineProps<{
  smartFolders: SmartFolder[]
}>()

const emit = defineEmits<{
  (e: 'apply', folder: SmartFolder): void
  (e: 'remove', id: number): void
}>()
</script>

<template>
  <div v-if="smartFolders.length > 0" class="smart-folders-bar">
    <span class="smart-folder-label">⭐ Smart Folders:</span>

    <div
      v-for="sf in smartFolders"
      :key="sf.id"
      class="smart-folder-pill"
      @click="emit('apply', sf)"
    >
      <span class="sf-name">📁 {{ sf.name }}</span>
      <!-- 后端动态匹配数；use_vector 且为 0 时表示尚未统计完 -->
      <span class="sf-count-badge">
        (Num: {{ sf.match_count === 0 && sf.use_vector ? '?' : sf.match_count }} )
      </span>
      <span class="sf-delete" @click.stop="emit('remove', sf.id)">✕</span>
    </div>
  </div>
</template>

<style scoped>
.smart-folders-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  margin-bottom: 20px;
  flex-wrap: wrap;
}

.smart-folder-label {
  font-size: 12px;
  color: #888;
  font-weight: bold;
}

.smart-folder-pill {
  background: #14141f;
  border: 1px solid #333348;
  padding: 4px 10px;
  border-radius: 20px;
  font-size: 12px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
  transition: all 0.2s;
}
.smart-folder-pill:hover {
  border-color: #67e5e5;
  background: #1a1a2b;
}

.sf-count-badge {
  color: #888;
  font-size: 11px;
}

.sf-delete {
  color: #666;
  font-size: 10px;
  border-radius: 50%;
  padding: 0 4px;
}
.sf-delete:hover {
  color: #ff4444;
  background: rgba(255, 68, 68, 0.2);
}
</style>
