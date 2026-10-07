<script setup lang="ts">
defineProps<{
  open: boolean
  text: string
  source: string
}>()

const emit = defineEmits<{
  (e: 'update:text', value: string): void
  (e: 'update:source', value: string): void
  (e: 'close'): void
  (e: 'save'): void
}>()
</script>

<template>
  <div v-if="open" class="dialog-overlay" @click.self="emit('close')">
    <div class="dialog-panel">
      <h3 class="dialog-title">{{ $t('textEntry.title') }}</h3>
      <p class="dialog-hint">{{ $t('textEntry.hint') }}</p>

      <textarea
        :value="text"
        @input="emit('update:text', ($event.target as HTMLTextAreaElement).value)"
        :placeholder="$t('textEntry.placeholder')"
        class="entry-textarea"
        rows="6"
      ></textarea>

      <input
        :value="source"
        @input="emit('update:source', ($event.target as HTMLInputElement).value)"
        type="text"
        :placeholder="$t('textEntry.sourcePlaceholder')"
        class="entry-source"
      />

      <div class="dialog-actions">
        <button class="btn btn-secondary" @click="emit('close')">{{ $t('common.cancel') }}</button>
        <button class="btn btn-primary" :disabled="!text.trim()" @click="emit('save')">{{ $t('common.save') }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}
.dialog-panel {
  background: #12121a;
  border: 1px solid #2a2a3a;
  border-radius: 12px;
  padding: 22px;
  width: 480px;
  max-width: calc(100vw - 40px);
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.dialog-title {
  margin: 0;
  font-size: 17px;
  color: #fff;
}
.dialog-hint {
  margin: 0;
  font-size: 12px;
  color: #888;
  line-height: 1.5;
}
.entry-textarea {
  width: 100%;
  background: #0c0c12;
  color: #fff;
  border: 1px solid #2a2a3a;
  border-radius: 6px;
  padding: 10px;
  font-size: 13px;
  resize: vertical;
  box-sizing: border-box;
  outline: none;
}
.entry-textarea:focus,
.entry-source:focus {
  border-color: #8333ff;
}
.entry-source {
  width: 100%;
  background: #0c0c12;
  color: #fff;
  border: 1px solid #2a2a3a;
  border-radius: 6px;
  padding: 8px 10px;
  font-size: 12px;
  box-sizing: border-box;
  outline: none;
}
.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
