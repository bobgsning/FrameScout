<script setup lang="ts">
/**
 * ClipDialog — 切片段时长输入弹层。
 * 从 useClip 的单例状态读取 pendingItem；确认后调用 confirmClip（内部发后端命令）。
 */
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useClip } from '@/composables/useClip'

const { t } = useI18n()
const { pendingItem, duration, confirmClip, cancelClip } = useClip()

const input = ref('5')

watch(pendingItem, (item) => {
  if (item) input.value = String(duration.value)
})

function onConfirm() {
  const n = Number(input.value)
  duration.value = Number.isFinite(n) && n > 0 ? n : 5
  confirmClip()
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="pendingItem"
      class="clip-dialog-overlay"
      @click.self="cancelClip"
      @keyup.esc="cancelClip"
    >
      <div class="clip-dialog">
        <h3 class="clip-dialog-title">{{ t('clip.title') }}</h3>
        <label class="clip-dialog-label">{{ t('clip.durationLabel') }}</label>
        <input
          v-model="input"
          type="number"
          min="0.5"
          step="0.5"
          autofocus
          class="clip-dialog-input"
          @keyup.enter="onConfirm"
        />
        <div class="clip-dialog-actions">
          <button class="clip-btn clip-btn-secondary" @click="cancelClip">
            {{ t('common.cancel') }}
          </button>
          <button class="clip-btn clip-btn-primary" @click="onConfirm">
            {{ t('clip.confirm') }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.clip-dialog-overlay {
  position: fixed;
  inset: 0;
  z-index: 10000;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
}

.clip-dialog {
  width: 320px;
  max-width: calc(100vw - 48px);
  background: #1e1e2e;
  border: 1px solid rgba(108, 142, 227, 0.25);
  border-radius: 12px;
  padding: 20px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
}

.clip-dialog-title {
  margin: 0 0 14px 0;
  font-size: 16px;
  font-weight: 600;
  color: #f0f0f5;
}

.clip-dialog-label {
  display: block;
  margin-bottom: 8px;
  font-size: 13px;
  color: #8888a0;
}

.clip-dialog-input {
  width: 100%;
  box-sizing: border-box;
  padding: 9px 12px;
  background: #14141e;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 8px;
  color: #f0f0f5;
  font-size: 14px;
  outline: none;
  transition: border-color 0.15s;
}

.clip-dialog-input:focus {
  border-color: #6c8ee3;
}

.clip-dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 18px;
}

.clip-btn {
  padding: 8px 14px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.15s;
}

.clip-btn-secondary {
  background: rgba(255, 255, 255, 0.06);
  border-color: rgba(255, 255, 255, 0.12);
  color: #f0f0f5;
}

.clip-btn-secondary:hover {
  background: rgba(255, 255, 255, 0.12);
}

.clip-btn-primary {
  background: #4a6bc8;
  color: #fff;
}

.clip-btn-primary:hover {
  background: #6c8ee3;
}
</style>
