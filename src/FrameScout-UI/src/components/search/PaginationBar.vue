<script setup lang="ts">
import { ref, watch } from 'vue'

const props = defineProps<{
  currentPage: number
  totalPages: number
  totalResults: number
  isShowAllMode: boolean
}>()

const emit = defineEmits<{
  (e: 'change-page', delta: number): void
  (e: 'jump', page: number): void
  (e: 'show-all'): void
  (e: 'exit-show-all'): void
}>()

/** 跳转输入框独立维护，回车/GO 时才提交给父级 */
const jumpInput = ref(props.currentPage)

watch(
  () => props.currentPage,
  (val) => {
    jumpInput.value = val
  }
)

function submitJump() {
  emit('jump', jumpInput.value)
}
</script>

<template>
  <div v-if="props.totalResults > 0" class="pagination-wrapper">
    <!-- Show All 模式：只显示状态条与退出按钮 -->
    <div v-if="props.isShowAllMode" class="show-all-bar">
      <span class="show-all-info">
        📂 Showing all {{ props.totalResults }} files (no pagination)
      </span>
      <button class="btn btn-secondary btn-sm" @click="emit('exit-show-all')">
        📄 Paginated View
      </button>
    </div>

    <template v-else>
      <button
        class="btn btn-secondary btn-sm"
        :disabled="props.currentPage === 1"
        @click="emit('change-page', -1)"
      >
        Prev
      </button>

      <div class="page-info">
        <span class="page-current">Page {{ props.currentPage }} / {{ props.totalPages }}</span>
        <p class="page-total">(Total {{ props.totalResults }} hits)</p>
      </div>

      <button
        class="btn btn-show-all"
        title="Load all indexed files at once"
        @click="emit('show-all')"
      >
        📋 Show All
      </button>

      <button
        class="btn btn-secondary btn-sm"
        :disabled="props.currentPage >= props.totalPages"
        @click="emit('change-page', 1)"
      >
        Next
      </button>

      <div class="jump-box">
        <span>Jump to</span>
        <input
          v-model.number="jumpInput"
          @keyup.enter="submitJump"
          id="jumpPage"
          type="number"
          min="1"
          :max="props.totalPages"
          class="jump-input"
        />
        <button class="btn btn-primary btn-sm" @click="submitJump">GO</button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.pagination-wrapper {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 15px;
  margin-top: 40px;
  padding-bottom: 40px;
  flex-wrap: wrap;
}

.page-info {
  text-align: center;
}

.page-current {
  font-weight: bold;
  background: var(--grad-purple-cyan);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
}

.page-total {
  font-size: 12px;
  color: #888;
  margin: 2px 0 0 0;
}

.jump-box {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #aaa;
}

.jump-input {
  width: 45px;
  padding: 5px;
  text-align: center;
  background: #121218;
  border: 1px solid #333;
  color: #fff;
  border-radius: 4px;
}

.btn-show-all {
  background: var(--grad-champagne);
  color: #000;
  padding: 8px 16px;
  border-radius: 6px;
  border: none;
  font-weight: bold;
  cursor: pointer;
  transition: all 0.2s ease;
  box-shadow: 0 0 8px rgba(231, 238, 195, 0.3);
}
.btn-show-all:hover {
  transform: translateY(-2px);
  box-shadow: 0 0 12px rgba(231, 238, 195, 0.5);
}

.show-all-bar {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 15px;
  margin-top: 20px;
  padding: 12px 20px;
  background: #1a1a28;
  border-radius: 8px;
  border: 1px solid #333348;
}

.show-all-info {
  color: #ccc;
  font-size: 14px;
  font-weight: 500;
}
</style>
