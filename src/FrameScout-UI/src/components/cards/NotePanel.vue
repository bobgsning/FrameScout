<script setup lang="ts">
import type { SearchResult } from '@/types/search'
import { saveNote } from '@/utils/api'

const props = defineProps<{
  item: SearchResult
}>()

function onBlur() {
  saveNote(props.item.path, props.item.user_note)
}
</script>

<template>
  <div class="note-panel">
    <p class="note-label">📝 Personal Note (Markdown):</p>
    <textarea
      v-model="props.item.user_note"
      @blur="onBlur"
      placeholder="Enter notes..."
      class="custom-textarea"
    ></textarea>
  </div>
</template>
