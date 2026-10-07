<script setup lang="ts">
/**
 * NotePanel — 笔记面板（P1-13 / 第三轮笔记与标注）。
 *
 * 第三轮修复：
 *   - 宣称 Markdown 实为纯 textarea → 现用 marked 渲染，preview/edit 双态切换；
 *   - 视频笔记无法精确到帧 → saveNote 传 timestamp（视频帧级标注）；
 *   - 保存失败用户完全不可见 → 加成功/失败 Toast 反馈 + 脏标记。
 */
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { marked } from 'marked'
import type { SearchResult } from '@/types/search'
import { saveNote } from '@/utils/api'
import { useToast } from '@/composables/useToast'

const props = defineProps<{
  item: SearchResult
}>()

const { push } = useToast()
const { t } = useI18n()

// 编辑/预览双态
const isEditing = ref(false)
// 脏标记：有未保存的修改
const isDirty = ref(false)
// 保存中
const isSaving = ref(false)
// P1-4：编辑前快照，取消/Esc 时还原
const editSnapshot = ref('')

// 渲染后的 Markdown HTML
const renderedNote = computed(() => {
  if (!props.item.user_note) return ''
  try {
    return marked.parse(props.item.user_note, { breaks: true }) as string
  } catch {
    return props.item.user_note
  }
})

// 切换到编辑态
function startEdit() {
  editSnapshot.value = props.item.user_note || ''
  isEditing.value = true
}

// P1-4：取消编辑，还原到编辑前的值
function cancelEdit() {
  props.item.user_note = editSnapshot.value
  isEditing.value = false
  isDirty.value = false
}

// P1-4：删除笔记（清空并落库）
async function deleteNote() {
  const ok = await saveNote(props.item.path, '', props.item.timestamp)
  if (ok) {
    props.item.user_note = ''
    isDirty.value = false
    isEditing.value = false
    push(t('cards.noteDeleted'), 'success', 2000)
  }
}

// 失焦保存
async function onBlur() {
  if (!isDirty.value) {
    isEditing.value = false
    return
  }
  await doSave()
}

// 手动保存
async function doSave() {
  if (isSaving.value) return
  isSaving.value = true
  // P1-13：视频帧级笔记传 timestamp（图片 timestamp=0 不影响）
  const ok = await saveNote(props.item.path, props.item.user_note, props.item.timestamp)
  isSaving.value = false
  if (ok) {
    isDirty.value = false
    isEditing.value = false
    push(t('cards.noteSaved'), 'success', 2000)
  } else {
    // saveNote 内部已 Toast 报错，这里只保持编辑态
  }
}

// 监听内容变化标记脏
watch(() => props.item.user_note, () => {
  isDirty.value = true
})
</script>

<template>
  <div class="note-panel">
    <div class="note-header">
      <p class="note-label">{{ $t('cards.notes') }} <span v-if="isDirty" class="dirty-dot" :title="$t('cards.unsavedChanges')">●</span></p>
      <div class="note-header-actions">
        <button v-if="!isEditing && props.item.user_note" class="btn-edit" @click="startEdit" :title="$t('cards.edit')">✏️</button>
        <button v-if="!isEditing && props.item.user_note" class="btn-delete" @click="deleteNote" :title="$t('cards.deleteNote')">🗑</button>
      </div>
    </div>

    <!-- 预览态：渲染 Markdown -->
    <div
      v-if="!isEditing && props.item.user_note"
      class="note-preview"
      v-html="renderedNote"
      @dblclick="startEdit"
    ></div>

    <!-- 空笔记提示 -->
    <p v-if="!isEditing && !props.item.user_note" class="note-empty-hint" @click="startEdit">
      {{ $t('cards.notePlaceholder') }}
    </p>

    <!-- 编辑态 -->
    <textarea
      v-if="isEditing"
      v-model="props.item.user_note"
      @blur="onBlur"
      @keydown.esc="cancelEdit"
      :placeholder="$t('cards.noteInputPlaceholder')"
      class="custom-textarea"
      autofocus
    ></textarea>
    <!-- P1-4：编辑态操作（取消还原 / 保存） -->
    <div v-if="isEditing" class="note-edit-actions">
      <button class="btn-cancel" @click="cancelEdit">{{ $t('common.cancel') }}</button>
      <button class="btn-save" @click="doSave" :disabled="isSaving">{{ $t('common.save') }}</button>
    </div>

    <!-- 帧级时间戳提示 -->
    <p v-if="props.item.timestamp > 0" class="note-frame-hint">
      {{ $t('cards.noteAtTime', { time: props.item.timestamp.toFixed(1) }) }}
    </p>
  </div>
</template>

<style scoped>
.note-panel {
  margin-top: 8px;
}

.note-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.note-header-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.note-label {
  font-size: 12px;
  color: #8888a0;
  margin: 0;
}

.dirty-dot {
  color: #fbbf24;
  font-size: 10px;
}

.btn-edit {
  background: none;
  border: none;
  color: #666677;
  cursor: pointer;
  font-size: 12px;
  padding: 2px 6px;
  border-radius: 4px;
  transition: all 0.15s;
}

.btn-edit:hover {
  color: #6c8ee3;
  background: rgba(108, 142, 227, 0.1);
}

.btn-delete {
  background: none;
  border: none;
  color: #666677;
  cursor: pointer;
  font-size: 12px;
  padding: 2px 6px;
  border-radius: 4px;
  transition: all 0.15s;
}
.btn-delete:hover {
  color: #f87171;
  background: rgba(248, 113, 113, 0.1);
}

.note-edit-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 6px;
}
.btn-cancel,
.btn-save {
  padding: 5px 12px;
  border: 1px solid #333344;
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
  background: #1e1e28;
  color: #e8e8f0;
}
.btn-cancel:hover {
  background: #2a2a36;
}
.btn-save {
  background: #2b4b7a;
  border-color: #3a5f95;
}
.btn-save:hover:not(:disabled) {
  filter: brightness(1.1);
}
.btn-save:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.note-preview {
  font-size: 13px;
  color: #b0b0c0;
  line-height: 1.5;
  padding: 8px 10px;
  background: rgba(10, 10, 12, 0.4);
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.05);
  cursor: pointer;
  min-height: 24px;
}

.note-preview :deep(h1),
.note-preview :deep(h2),
.note-preview :deep(h3) {
  color: #f0f0f5;
  margin: 8px 0 4px 0;
}

.note-preview :deep(h1) { font-size: 16px; }
.note-preview :deep(h2) { font-size: 14px; }
.note-preview :deep(h3) { font-size: 13px; }

.note-preview :deep(code) {
  background: rgba(108, 142, 227, 0.15);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 12px;
}

.note-preview :deep(pre) {
  background: rgba(0, 0, 0, 0.3);
  padding: 8px;
  border-radius: 4px;
  overflow-x: auto;
}

.note-preview :deep(pre code) {
  background: none;
  padding: 0;
}

.note-preview :deep(ul),
.note-preview :deep(ol) {
  padding-left: 20px;
  margin: 4px 0;
}

.note-preview :deep(a) {
  color: #6c8ee3;
  text-decoration: none;
}

.note-preview :deep(a:hover) {
  text-decoration: underline;
}

.note-preview :deep(blockquote) {
  border-left: 3px solid #6c8ee3;
  padding-left: 10px;
  margin: 4px 0;
  color: #8888a0;
}

.note-empty-hint {
  font-size: 12px;
  color: #555566;
  padding: 8px 10px;
  background: rgba(10, 10, 12, 0.3);
  border-radius: 6px;
  border: 1px dashed rgba(255, 255, 255, 0.08);
  cursor: pointer;
  margin: 0;
  transition: all 0.15s;
}

.note-empty-hint:hover {
  border-color: rgba(108, 142, 227, 0.3);
  color: #8888a0;
}

.note-frame-hint {
  font-size: 11px;
  color: #555566;
  margin: 4px 0 0 0;
}
</style>
