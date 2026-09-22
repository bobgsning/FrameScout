/**
 * useLicense — Pro/Trial 授权状态与激活流程。
 * 独立无依赖；Esc 关闭弹窗的键盘监听随本 composable 生命周期注册/卸载。
 */
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { LicenseStatus } from '@/types/license'

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

export function useLicense() {
  async function checkLicense() {
    try {
      const res = await invoke<LicenseStatus>('get_license_status')
      licenseStatus.value = res
    } catch (e) {
      console.error(e)
    }
  }

  async function submitActivation() {
    if (!licenseEmail.value || !licenseKeyInput.value) return
    try {
      const resMsg: string = await invoke('activate_pro_license', {
        email: licenseEmail.value,
        licenseKey: licenseKeyInput.value
      })
      activateSuccess.value = true
      activateMsg.value = resMsg
      await checkLicense()
      setTimeout(() => {
        showActivateModal.value = false
      }, 1500)
    } catch (err: any) {
      activateSuccess.value = false
      activateMsg.value = String(err)
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
    submitActivation
  }
}
