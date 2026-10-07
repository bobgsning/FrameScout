<script setup lang="ts">
/**
 * ContextMenu — 通用右键上下文菜单（P1-1 / P1-8 配套）。
 *
 * 用法：父组件 @contextmenu.prevent="open($event, item)"，
 * 把菜单定位到鼠标位置；点遮罩或菜单项后关闭。
 * 菜单项由父组件通过 props 传入，点击后 emit('select', item)。
 */
import { ref, watch, nextTick } from 'vue'

export interface MenuItem {
  key: string
  label: string
  icon?: string
  /** 是否禁用（灰显） */
  disabled?: boolean
  /** 是否危险动作（红色） */
  danger?: boolean
  /** P2-7：快捷键提示列 */
  shortcut?: string
}

const props = defineProps<{
  visible: boolean
  items: MenuItem[]
  /** 菜单左上角坐标（相对视口） */
  x: number
  y: number
}>()

const emit = defineEmits<{
  select: [item: MenuItem]
  close: []
}>()

function onSelect(item: MenuItem) {
  if (item.disabled) return
  emit('select', item)
  emit('close')
}

// P2-7：视口溢出校正——菜单靠边时向左/向上翻转，避免被窗口裁掉
const menuEl = ref<HTMLElement | null>(null)
const pos = ref({ x: 0, y: 0 })
watch(
  () => props.visible,
  async (v) => {
    if (!v) return
    await nextTick()
    pos.value = { x: props.x, y: props.y }
    if (!menuEl.value) return
    const rect = menuEl.value.getBoundingClientRect()
    const vw = window.innerWidth
    const vh = window.innerHeight
    let x = props.x
    let y = props.y
    if (x + rect.width > vw) x = Math.max(8, vw - rect.width - 8)
    if (y + rect.height > vh) y = Math.max(8, vh - rect.height - 8)
    pos.value = { x, y }
  }
)
</script>

<template>
  <Teleport to="body">
    <div v-if="props.visible" class="ctx-overlay" @click="emit('close')" @contextmenu.prevent="emit('close')">
      <div
        ref="menuEl"
        class="ctx-menu"
        :style="{ left: pos.x + 'px', top: pos.y + 'px' }"
        @click.stop
      >
        <button
          v-for="item in props.items"
          :key="item.key"
          class="ctx-item"
          :class="{ disabled: item.disabled, danger: item.danger }"
          :disabled="item.disabled"
          @click="onSelect(item)"
        >
          <span v-if="item.icon" class="ctx-icon">{{ item.icon }}</span>
          <span class="ctx-label">{{ item.label }}</span>
          <span v-if="item.shortcut" class="ctx-shortcut">{{ item.shortcut }}</span>
        </button>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.ctx-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  background: transparent;
}

.ctx-menu {
  position: fixed;
  min-width: 200px;
  background: rgba(20, 20, 30, 0.96);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border: 1px solid rgba(108, 142, 227, 0.25);
  border-radius: 10px;
  padding: 6px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(0, 0, 0, 0.3);
  z-index: 10000;
  animation: ctx-fade-in 0.12s ease-out;
}

@keyframes ctx-fade-in {
  from { opacity: 0; transform: scale(0.96) translateY(-4px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}

.ctx-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px 14px;
  background: transparent;
  border: none;
  border-radius: 6px;
  color: #f0f0f5;
  font-size: 13px;
  font-family: 'Noto Sans', system-ui, sans-serif;
  cursor: pointer;
  text-align: left;
  transition: background 0.12s, color 0.12s;
}

.ctx-item:hover:not(.disabled) {
  background: rgba(108, 142, 227, 0.18);
  color: #fff;
}

.ctx-item.disabled {
  color: #555566;
  cursor: not-allowed;
}

.ctx-item.danger {
  color: #f87171;
}

.ctx-item.danger:hover:not(.disabled) {
  background: rgba(248, 113, 113, 0.15);
  color: #fca5a5;
}

.ctx-icon {
  font-size: 15px;
  flex-shrink: 0;
  width: 20px;
  text-align: center;
}

.ctx-label {
  flex: 1;
}

.ctx-shortcut {
  font-size: 11px;
  color: #666677;
  flex-shrink: 0;
  margin-left: 12px;
}
</style>
