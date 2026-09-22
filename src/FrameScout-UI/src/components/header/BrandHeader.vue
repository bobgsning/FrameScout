<script setup lang="ts">
defineProps<{
  isPro: boolean
}>()

const emit = defineEmits<{
  (e: 'open-license'): void
}>()
</script>

<template>
  <div class="brand-header-box">
    <svg width="36" height="36" viewBox="0 0 1000 1000" fill="none">
      <use href="#framescout-logo" />
    </svg>

    <h1 class="brand-header">FrameScout — Offline AI Search: Global Edition</h1>

    <!-- License status button (right-aligned) -->
    <div class="license-badge" @click="emit('open-license')">
      <span v-if="isPro" class="badge-pro">💎 Pro Edition</span>
      <span v-else class="badge-trial">⚡ Trial Mode</span>
    </div>
  </div>
</template>

<style scoped>
.brand-header-box {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-bottom: 20px;
  position: relative;
}

.brand-header {
  text-align: center;
  font-size: 36px;
  font-weight: 800;
  background: var(--grad-purple-cyan);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
}

/* License Status Button */
.license-badge {
  display: inline-flex;
  align-items: center;
  cursor: pointer;
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.3px;
  transition: all 0.25s ease;
  user-select: none;
  white-space: nowrap;
  margin-left: auto;
  padding: 0;
  border-radius: 0;
  background: none;
  border: none;
}

.badge-pro {
  background: linear-gradient(135deg, #8333ff, #67e5e5);
  color: #000;
  box-shadow: 0 0 12px rgba(131, 51, 255, 0.4);
  border: 1px solid rgba(131, 51, 255, 0.6);
  border-radius: 999px;
  padding: 6px 20px;
  font-size: 13px;
  line-height: 1.4;
  display: inline-block;
}
.badge-pro:hover {
  transform: scale(1.05);
  box-shadow: 0 0 20px rgba(131, 51, 255, 0.6);
}

.badge-trial {
  background: #1e1e2a;
  color: #ffaa33;
  border: 1px solid #ffaa33;
  box-shadow: 0 0 8px rgba(255, 170, 51, 0.2);
  animation: trialPulse 3s infinite ease-in-out;
  border-radius: 999px;
  padding: 6px 20px;
  font-size: 13px;
  line-height: 1.4;
  display: inline-block;
}
.badge-trial:hover {
  background: #2a2a3a;
  border-color: #ffbb44;
  box-shadow: 0 0 14px rgba(255, 170, 51, 0.4);
  transform: scale(1.05);
}

@media (max-width: 768px) {
  .brand-header-box {
    flex-wrap: wrap;
    justify-content: center;
  }
  .license-badge {
    margin-left: 0;
    margin-top: 8px;
  }
}
</style>
