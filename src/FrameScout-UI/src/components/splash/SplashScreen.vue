<script setup lang="ts">
import type { EngineStatusValue } from '@/types/license'

defineProps<{
  status: EngineStatusValue
  message: string
  retry: number
  maxRetries: number
}>()
</script>

<template>
  <div class="neural-splash">
    <div class="logo-wrapper">
      <svg width="80" height="80" viewBox="0 0 1000 1000" fill="none" class="brand-svg-glow">
        <use href="#framescout-logo" />
      </svg>
    </div>

    <div class="loader-ring"></div>

    <h2 class="splash-title">FRAME SCOUT NEURAL LINK ESTABLISHING...</h2>
    <p class="splash-subtitle">
      Loading large SigLIP 2 &amp; EasyOCR models into high-dimensional memory, please wait...
    </p>
    <p class="splash-subtitle">{{ message }}</p>

    <p v-if="status === 'connecting'" class="splash-retry">
      Attempt {{ retry }} / {{ maxRetries }}
    </p>
    <p v-if="status === 'error'" class="splash-error">{{ message }}</p>
  </div>
</template>

<style scoped>
.neural-splash {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: #070709;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #67e5e5;
  font-family: monospace;
}

.logo-wrapper {
  animation: pulse 2s infinite ease-in-out;
  margin-bottom: 35px;
}

.brand-svg-glow {
  filter: drop-shadow(0 0 12px rgba(131, 51, 255, 0.6));
}

.loader-ring {
  border: 4px solid #1a1a24;
  border-top: 4px solid #8333ff;
  border-radius: 50%;
  width: 50px;
  height: 50px;
  animation: spin 1s linear infinite;
  margin-bottom: 20px;
  box-shadow: 0 0 15px rgba(131, 51, 255, 0.5);
}

.splash-title {
  background: var(--grad-purple-cyan);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
}

.splash-subtitle {
  color: #888;
  font-size: 14px;
}

.splash-retry {
  color: #ffaa33;
  font-size: 13px;
  margin-top: 4px;
}

.splash-error {
  color: #ff4455;
  font-size: 13px;
  margin-top: 8px;
}
</style>
