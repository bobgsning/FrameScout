<script setup lang="ts">
import { ref, watch } from 'vue'

const props = defineProps<{
  currentPage: number
  totalPages: number
  totalResults: number
  isShowAllMode: boolean
  /**
   * 是否显示「📋 Show All」按钮。
   * 该按钮的语义是「一次性加载全部已索引媒体」，仅适用于媒体浏览；
   * 纯文本结果分页与管理面板传 false 隐藏它。
   */
  showAllButton?: boolean
  /** 紧凑模式：用于对话框等空间受限的场景，去掉默认的外边距 */
  dense?: boolean
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
  <div
    v-if="props.totalResults > 0"
    class="pagination-wrapper"
    :class="{ 'pagination-dense': props.dense }"
  >
    <!-- Show All 模式：只显示状态条与退出按钮 -->
    <div v-if="props.isShowAllMode" class="show-all-bar">
      <span class="show-all-info">
        {{ $t('pagination.showingAll', { total: props.totalResults }) }}
      </span>
      <button class="btn btn-secondary btn-sm" @click="emit('exit-show-all')">
        {{ $t('pagination.paginatedView') }}
      </button>
    </div>

    <template v-else>
      <button
        class="btn btn-secondary btn-sm"
        :disabled="props.currentPage === 1"
        @click="emit('change-page', -1)"
      >
        {{ $t('pagination.prev') }}
      </button>

      <div class="page-info">
        <span class="page-current">{{ $t('pagination.page', { current: props.currentPage, total: props.totalPages }) }}</span>
        <p class="page-total">{{ $t('pagination.totalHits', { total: props.totalResults }) }}</p>
      </div>

      <button
        v-if="props.showAllButton !== false"
        class="btn btn-show-all"
        :title="$t('pagination.showAllHint')"
        @click="emit('show-all')"
      >
        {{ $t('pagination.showAll') }}
      </button>

      <button
        class="btn btn-secondary btn-sm"
        :disabled="props.currentPage >= props.totalPages"
        @click="emit('change-page', 1)"
      >
        {{ $t('pagination.next') }}
      </button>

      <div class="jump-box">
        <span>{{ $t('pagination.jumpTo') }}</span>
        <input
          v-model.number="jumpInput"
          @keyup.enter="submitJump"
          id="jumpPage"
          type="number"
          min="1"
          :max="props.totalPages"
          class="jump-input"
        />
        <button class="btn btn-primary btn-sm" @click="submitJump">{{ $t('pagination.go') }}</button>
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

/* 紧凑模式：对话框 / 内嵌列表里不需要那么大的上下留白 */
.pagination-dense {
  margin-top: 10px;
  padding-bottom: 4px;
  gap: 10px;
  font-size: 12px;
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
