<script setup lang="ts">
import type { LicenseStatus } from '@/types/license'
import { useLicense } from '@/composables/useLicense'

const props = defineProps<{
  licenseStatus: LicenseStatus
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

// 表单字段由 useLicense 单例持有（App.vue 已初始化，此处只取引用）
const { licenseEmail, licenseKeyInput, activateMsg, activateSuccess, submitActivation } =
  useLicense()

// submitActivation 内部已 checkLicense + setTimeout 关闭弹窗，无需再额外触发
async function handleSubmit() {
  await submitActivation()
}
</script>

<template>
  <div class="modal-overlay" @click.self="emit('close')">
    <div
      class="modal-card"
      :class="props.licenseStatus.is_pro ? 'pro-theme' : 'trial-theme'"
      role="dialog"
      aria-modal="true"
    >
      <h2>
        {{ props.licenseStatus.is_pro ? '💎' : '⚡' }}
        {{ props.licenseStatus.is_pro ? 'Your License - Pro' : 'Activate FrameScout Pro' }}
      </h2>
      <p class="modal-sub">Enter your email and the License Key provided after purchase.</p>

      <div class="form-group">
        <label>Email Address</label>
        <input
          v-model="licenseEmail"
          type="email"
          placeholder="your@email.com"
          class="custom-input modal-input"
        />
      </div>

      <div class="form-group">
        <label>License Key</label>
        <textarea
          v-model="licenseKeyInput"
          placeholder="Paste your Base64 License Key here..."
          class="custom-textarea modal-textarea"
        ></textarea>
      </div>

      <p v-if="activateMsg" :class="['activate-msg', activateSuccess ? 'msg-success' : 'msg-error']">
        {{ activateMsg }}
      </p>

      <div class="modal-actions">
        <button class="btn btn-secondary" @click="emit('close')">Cancel</button>
        <button class="btn btn-primary" @click="handleSubmit">Activate License</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  animation: fadeIn 0.2s ease;
}

.modal-card {
  background: #12121a;
  border: 1px solid var(--modal-border-color);
  border-radius: 16px;
  padding: 32px 28px;
  max-width: 460px;
  width: 90%;
  box-shadow: var(--modal-shadow);
  color: #ddd;
  position: relative;
  animation: scaleIn 0.25s ease;
}

.modal-card h2 {
  font-size: 22px;
  font-weight: 800;
  background: var(--modal-title-gradient);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  margin: 0 0 8px 0;
  text-align: center;
}

.modal-sub {
  font-size: 13px;
  color: #888;
  text-align: center;
  margin: 0 0 24px 0;
  line-height: 1.4;
}

.form-group {
  margin-bottom: 18px;
}

.form-group label {
  display: block;
  font-size: 12px;
  color: #aaa;
  margin-bottom: 6px;
  font-weight: 600;
  letter-spacing: 0.5px;
}

.modal-card .btn-primary {
  background: var(--modal-btn-primary-bg);
  color: #000;
  box-shadow: var(--modal-btn-primary-shadow);
  border: none;
}
.modal-card .btn-primary:hover {
  opacity: 0.95;
  transform: translateY(-1px);
}

.modal-input,
.modal-textarea {
  width: 100%;
  box-sizing: border-box;
  background: #0c0c12;
  border: 1px solid #2a2a3a;
  border-radius: 8px;
  padding: 12px 14px;
  color: #eee;
  font-size: 14px;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.modal-input:focus,
.modal-textarea:focus {
  border-color: var(--modal-input-focus-border);
  box-shadow: var(--modal-input-focus-shadow);
}

.modal-textarea {
  min-height: 70px;
  resize: vertical;
  font-family: monospace;
  font-size: 13px;
}

.activate-msg {
  text-align: center;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
  margin: 16px 0 0 0;
}

.msg-success {
  background: var(--modal-success-bg);
  color: var(--modal-success-color);
  border: 1px solid var(--modal-success-border);
}

.msg-error {
  background: var(--modal-error-bg);
  color: var(--modal-error-color);
  border: 1px solid var(--modal-error-border);
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 24px;
}

.modal-actions .btn {
  padding: 10px 24px;
  font-size: 14px;
  border-radius: 8px;
}
</style>
