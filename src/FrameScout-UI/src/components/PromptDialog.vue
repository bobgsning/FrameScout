<script setup lang="ts">
/**
 * PromptDialog — 通用文本输入对话框（P1-3 / P1-8 配套）。
 *
 * 替代原生 window.prompt：支持中文化、长度校验、视觉与应用内对话框统一。
 * 用法：父组件 v-model:visible 控制，@confirm 拿到输入值。
 */
import { ref, watch, onUnmounted } from 'vue'

const props = defineProps<{
  visible: boolean
  title: string
  label?: string
  placeholder?: string
  /** 默认值（如当前搜索词） */
  defaultValue?: string
  /** 最大长度 */
  maxLength?: number
  confirmText?: string
  /** P2-11：多行模式（textarea），默认单行 */
  multiline?: boolean
}>()

const emit = defineEmits<{
  confirm: [value: string]
  cancel: []
  'update:visible': [v: boolean]
}>()

const inputValue = ref('')

// visible 变为 true 时，用 defaultValue 初始化
watch(() => props.visible, (v) => {
  if (v) {
    inputValue.value = props.defaultValue || ''
  }
})

function onConfirm() {
  const trimmed = inputValue.value.trim()
  if (!trimmed) return
  emit('confirm', trimmed)
  emit('update:visible', false)
}

function onCancel() {
  emit('cancel')
  emit('update:visible', false)
}

// P2-11：window 级键盘监听——此前 Esc/Enter 依赖弹窗内焦点（Teleport 场景不可靠）
function onWindowKeydown(e: KeyboardEvent) {
  if (!props.visible) return
  if (e.key === 'Escape') {
    e.stopPropagation()
    onCancel()
  } else if (e.key === 'Enter' && !props.multiline) {
    // 多行模式下 Enter 是换行，不触发确认
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
    <div v-if="props.visible" class="prompt-overlay" @click.self="onCancel">
      <div class="prompt-dialog">
        <h3 class="prompt-title">{{ props.title }}</h3>
        <label v-if="props.label" class="prompt-label">{{ props.label }}</label>
        <textarea
          v-if="props.multiline"
          v-model="inputValue"
          class="prompt-input prompt-textarea"
          :placeholder="props.placeholder"
          :maxlength="props.maxLength"
          rows="5"
          autofocus
        ></textarea>
        <input
          v-else
          v-model="inputValue"
          type="text"
          class="prompt-input"
          :placeholder="props.placeholder"
          :maxlength="props.maxLength"
          autofocus
        />
        <p v-if="props.maxLength" class="prompt-count">
          {{ inputValue.length }} / {{ props.maxLength }}
        </p>
        <div class="prompt-actions">
          <button class="btn-cancel" @click="onCancel">{{ $t('common.cancel') }}</button>
          <button class="btn-confirm" @click="onConfirm" :disabled="!inputValue.trim()">
            {{ props.confirmText || $t('common.confirm') }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.prompt-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
}

.prompt-dialog {
  width: 420px;
  max-width: 90vw;
  padding: 28px;
  background: linear-gradient(180deg, rgba(28, 28, 46, 0.98), rgba(20, 20, 30, 0.98));
  border: 1px solid rgba(108, 142, 227, 0.3);
  border-radius: 14px;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
}

.prompt-title {
  margin: 0 0 16px 0;
  font-size: 16px;
  font-weight: 600;
  color: #f0f0f5;
}

.prompt-label {
  display: block;
  margin-bottom: 6px;
  font-size: 13px;
  color: #8888a0;
}

.prompt-input {
  width: 100%;
  box-sizing: border-box;
  padding: 12px 14px;
  background: rgba(10, 10, 12, 0.6);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  color: #f0f0f5;
  font-size: 14px;
  font-family: 'Noto Sans', system-ui, sans-serif;
  outline: none;
  transition: border-color 0.15s;
}

.prompt-input:focus {
  border-color: rgba(108, 142, 227, 0.5);
}

.prompt-textarea {
  resize: vertical;
  min-height: 90px;
  line-height: 1.5;
}

.prompt-count {
  margin: 6px 0 0 0;
  font-size: 11px;
  color: #666677;
  text-align: right;
}

.prompt-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 20px;
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
.btn-confirm:hover:not(:disabled) {
  filter: brightness(1.1);
}
.btn-confirm:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
