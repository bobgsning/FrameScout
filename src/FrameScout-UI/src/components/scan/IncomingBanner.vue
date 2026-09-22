<script setup lang="ts">
const props = defineProps<{
  count: number
}>()

const emit = defineEmits<{
  (e: 'accept'): void
  (e: 'dismiss'): void
}>()
</script>

<template>
  <div class="incoming-banner" @click="emit('accept')">
    <span class="banner-text">
      📥 {{ props.count }} new {{ props.count === 1 ? 'image has' : 'images have' }} been indexed.
      Click to view.
    </span>
    <span class="banner-close" @click.stop="emit('dismiss')">✕</span>
  </div>
</template>

<style scoped>
.incoming-banner {
  position: sticky;
  top: 0;
  z-index: 10;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: var(--grad-purple-cyan);
  color: #000;
  padding: 12px 20px;
  border-radius: 8px;
  cursor: pointer;
  font-weight: bold;
  margin: 10px auto;
  max-width: 95%;
  width: fit-content;
  box-shadow: 0 0 15px rgba(131, 51, 255, 0.4);
  transition: transform 0.2s ease, box-shadow 0.2s ease;
  animation: slideDown 0.3s ease-out;
}
.incoming-banner:hover {
  transform: scale(1.03);
  box-shadow: 0 0 25px rgba(131, 51, 255, 0.6);
}

.banner-close {
  cursor: pointer;
  font-size: 16px;
  opacity: 0.7;
  margin-left: 10px;
}
.banner-close:hover {
  opacity: 1;
}
</style>
