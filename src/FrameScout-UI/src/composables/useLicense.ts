/**
 * useLicense — Pro/Trial 授权状态与激活流程。
 * 独立无依赖；Esc 关闭弹窗的键盘监听随本 composable 生命周期注册/卸载。
 */
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { LicenseStatus } from '@/types/license'
import { useToast } from './useToast'
import { t } from '@/i18n'
import { localizeBackend } from '@/utils/backendText'

const licenseStatus = ref<LicenseStatus>({ is_pro: false, email: '', limit: 100 })
const showActivateModal = ref(false)
const licenseEmail = ref('')
const licenseKeyInput = ref('')
const activateMsg = ref('')
const activateSuccess = ref(false)

let lifecycleBound = false

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && showActivateModal.value) {
    showActivateModal.value = false
  }
}

/** 把到期时间戳格式化为本地日期（如 2026-10-29）。 */
function formatExpiry(ts: number): string {
  const d = new Date(ts * 1000)
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

/**
 * 把后端授权错误映射成友好文案。
 *
 * 后端现在只回 `ERR_LICENSE_*` 码（此前是英文整句，靠下面的子串匹配猜类别），
 * 故直接交给 localizeBackend 按 i18n 的 `backend.*` 解析。
 * 若拿到的不是码（旧版本后端或第三方错误），退回通用提示并附上原文。
 */
function friendlyActivationError(err: unknown): string {
  const raw = String(err ?? '')
  const localized = localizeBackend(err)
  if (localized !== raw) return localized
  return t('license.genericError', { err: raw })
}

export function useLicense() {
  async function checkLicense() {
    try {
      const res = await invoke<LicenseStatus>('get_license_status')
      licenseStatus.value = res
    } catch (e) {
      // P1-10：授权状态拉取失败此前静默；现给可见反馈
      const { push } = useToast()
      push(t('license.statusFailed', { err: e }), 'error')
    }
  }

  async function submitActivation() {
    if (!licenseEmail.value || !licenseKeyInput.value) return
    try {
      await invoke<string>('activate_pro_license', {
        email: licenseEmail.value,
        licenseKey: licenseKeyInput.value
      })
      activateSuccess.value = true
      // 刷新状态拿到到期日，拼成友好成功消息
      await checkLicense()
      const exp = licenseStatus.value.expiry
      activateMsg.value = exp != null
        ? t('license.activatedUntil', { date: formatExpiry(exp) })
        : t('license.activated')
      setTimeout(() => {
        showActivateModal.value = false
      }, 1800)
    } catch (err: unknown) {
      activateSuccess.value = false
      activateMsg.value = friendlyActivationError(err)
    }
  }

  // useLicense 也会被 LicenseModal 调用，守卫确保键盘监听与初始拉取只发生一次
  if (!lifecycleBound) {
    lifecycleBound = true

    onMounted(() => {
      document.addEventListener('keydown', handleKeydown)
      checkLicense()
    })

    onUnmounted(() => {
      document.removeEventListener('keydown', handleKeydown)
      // HMR：允许重新挂载时再次注册 Esc 监听与首次拉取授权
      lifecycleBound = false
    })
  }

  return {
    licenseStatus,
    showActivateModal,
    licenseEmail,
    licenseKeyInput,
    activateMsg,
    activateSuccess,
    checkLicense,
    submitActivation,
    formatExpiry
  }
}
