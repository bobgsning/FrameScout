<script setup lang="ts">
/**
 * ConfirmDialog — 应用内确认对话框（第五轮 P0-4 / P1：替代原生 window.confirm）。
 *
 * 与 PromptDialog（文本输入）语义互补：这里用于「破坏性操作的二次确认」，
 * 返回 confirm / cancel 事件，不做文本输入。危险操作传 danger 显示红色按钮。
 *
 * 用法：父组件 v-model:visible 控制，@confirm 执行破坏性操作。
 * 键盘：Esc = 取消、Enter = 确认（window 级监听，不依赖弹窗内焦点）。
 */
import { watch, onUnmounted } from 'vue'

const props = withDefaults(
  defineProps<{
    visible: boolean
    title: string
    message: string
    /** 补充信息（如条目数 / 「不可回退」警告） */
    detail?: string
    confirmText?: string
    cancelText?: string
    /** 危险操作（红色按钮），如 purge / 终态删除 */
    danger?: boolean
  }>(),
  { danger: false }
)

const emit = defineEmits<{
  confirm: []
  cancel: []
  'update:visible': [v: boolean]
}>()

function onConfirm() {
  emit('confirm')
  emit('update:visible', false)
}

function onCancel() {
  emit('cancel')
  emit('update:visible', false)
}

// window 级键盘监听（capture + stopPropagation），打开时 Esc/Enter 直接处理，
// 不依赖弹窗内焦点，也不冒泡到 useKeyboard 的全局 Esc。
function onWindowKeydown(e: KeyboardEvent) {
  if (!props.visible) return
  if (e.key === 'Escape') {
    e.stopPropagation()
    onCancel()
  } else if (e.key === 'Enter') {
    e.stopPropagation()
    onConfirm()
  }
}

watch(
  () => props.visible,
  (v) => {
    if (v) window.addEventListener('keydown', onWindowKeydown, true)
    else window.removeEventListener('keydown', onWindowKeydown, true)
  }
)

onUnmounted(() => window.removeEventListener('keydown', onWindowKeydown, true))
</script>

<template>
  <Teleport to="body">
    <div v-if="props.visible" class="confirm-overlay" @click.self="onCancel">
      <div class="confirm-dialog">
        <h3 class="confirm-title">{{ props.title }}</h3>
        <p class="confirm-message">{{ props.message }}</p>
        <p v-if="props.detail" class="confirm-detail">{{ props.detail }}</p>
        <div class="confirm-actions">
          <button class="btn-cancel" @click="onCancel">
            {{ props.cancelText || $t('common.cancel') }}
          </button>
          <button class="btn-confirm" :class="{ danger: props.danger }" @click="onConfirm">
            {{ props.confirmText || $t('common.confirm') }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.confirm-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
}

.confirm-dialog {
  width: 440px;
  max-width: 90vw;
  padding: 28px;
  background: linear-gradient(180deg, rgba(28, 28, 46, 0.98), rgba(20, 20, 30, 0.98));
  border: 1px solid rgba(108, 142, 227, 0.3);
  border-radius: 14px;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
}

.confirm-title {
  margin: 0 0 14px 0;
  font-size: 16px;
  font-weight: 600;
  color: #f0f0f5;
}

.confirm-message {
  margin: 0 0 8px 0;
  font-size: 14px;
  color: #d0d0dd;
  line-height: 1.6;
  word-break: break-word;
}

.confirm-detail {
  margin: 0;
  font-size: 12px;
  color: #f0a0a0;
  line-height: 1.5;
}

.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 22px;
}

.btn-cancel,
.btn-confirm {
  padding: 10px 20px;
  border: none;
  border-radius: 8px;
  font-size: 13px;
  font-family: 'Noto Sans', system-ui, sans-serif;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-cancel {
  background: rgba(255, 255, 255, 0.06);
  color: #8888a0;
}
.btn-cancel:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #f0f0f5;
}

.btn-confirm {
  background: linear-gradient(135deg, #6c8ee3, #4a6fcf);
  color: #fff;
  font-weight: 500;
}
.btn-confirm:hover {
  filter: brightness(1.1);
}
.btn-confirm.danger {
  background: linear-gradient(135deg, #f87171, #dc2626);
}
</style>
