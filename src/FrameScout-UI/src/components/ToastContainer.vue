<script setup lang="ts">
/**
 * ToastContainer — 全局 Toast 通知容器（P1-8 / 第三轮「无全局 Toast 体系」）。
 *
 * 右上角堆叠 + 自动消失 + 三色语义 + 玻璃拟态。
 * 由 useToast 驱动，App.vue 挂载一次即可。
 */
import { useToast } from '@/composables/useToast'

const { toasts, dismiss } = useToast()

const kindIcon: Record<string, string> = {
  success: '✅',
  error: '❌',
  info: 'ℹ️',
  warning: '⚠️',
}
</script>

<template>
  <Teleport to="body">
    <div class="toast-container">
      <TransitionGroup name="toast">
        <div
          v-for="t in toasts"
          :key="t.id"
          class="toast-item"
          :class="t.kind"
          @click="dismiss(t.id)"
        >
          <span class="toast-icon">{{ kindIcon[t.kind] || 'ℹ️' }}</span>
          <span class="toast-msg">{{ t.message }}</span>
          <button
            v-if="t.action"
            class="toast-action"
            @click.stop="t.action.onClick(); dismiss(t.id)"
          >{{ t.action.label }}</button>
          <button class="toast-close" @click.stop="dismiss(t.id)">✕</button>
          <span
            v-if="t.remaining !== undefined"
            class="toast-progress"
            :style="{ width: (t.remaining / t.duration) * 100 + '%' }"
          ></span>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-container {
  position: fixed;
  top: 20px;
  right: 20px;
  z-index: 10001;
  display: flex;
  flex-direction: column;
  gap: 10px;
  pointer-events: none;
  max-width: 400px;
}

.toast-item {
  position: relative;
  overflow: hidden;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-radius: 10px;
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border: 1px solid;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
  cursor: pointer;
  pointer-events: auto;
  min-width: 280px;
  max-width: 400px;
  transition: opacity 0.2s;
}

.toast-item.success {
  background: rgba(74, 222, 128, 0.12);
  border-color: rgba(74, 222, 128, 0.35);
  color: #86efac;
}
.toast-item.error {
  background: rgba(248, 113, 113, 0.12);
  border-color: rgba(248, 113, 113, 0.35);
  color: #fca5a5;
}
.toast-item.info {
  background: rgba(96, 165, 250, 0.12);
  border-color: rgba(96, 165, 250, 0.3);
  color: #93c5fd;
}
.toast-item.warning {
  background: rgba(251, 191, 36, 0.12);
  border-color: rgba(251, 191, 36, 0.35);
  color: #fcd34d;
}

.toast-icon {
  font-size: 16px;
  flex-shrink: 0;
}

.toast-msg {
  flex: 1;
  font-size: 13px;
  line-height: 1.4;
  word-break: break-word;
}

.toast-close {
  background: none;
  border: none;
  color: inherit;
  opacity: 0.5;
  cursor: pointer;
  font-size: 12px;
  padding: 0 4px;
  flex-shrink: 0;
  transition: opacity 0.15s;
}

.toast-close:hover {
  opacity: 1;
}

/* P2-1：操作按钮（撤销等） */
.toast-action {
  background: none;
  border: 1px solid currentColor;
  border-radius: 6px;
  color: inherit;
  font-size: 12px;
  padding: 4px 10px;
  cursor: pointer;
  flex-shrink: 0;
  opacity: 0.85;
  transition: opacity 0.15s, background 0.15s;
}
.toast-action:hover {
  opacity: 1;
  background: rgba(255, 255, 255, 0.1);
}

/* P2-6：剩余时间进度条 */
.toast-progress {
  position: absolute;
  bottom: 0;
  left: 0;
  height: 2px;
  background: currentColor;
  opacity: 0.5;
  transition: width 0.1s linear;
}

/* TransitionGroup 动画 */
.toast-enter-active,
.toast-leave-active {
  transition: all 0.3s ease;
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(60px);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(60px) scale(0.95);
}

.toast-move {
  transition: transform 0.3s ease;
}
</style>
