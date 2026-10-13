<script setup lang="ts">
/**
 * OcrLanguageList — OCR 识别语言多选列表（替代原下拉框）。
 *
 * 三档分组展示：
 *   - 内置识别（en/ch_sim 随 wheel、ch_tra/ja 随安装包，后者可删）
 *   - 可下载识别（ru/ko/de/fr/es，模型按需下载）
 *   - 即将推出（pt/it/pl/tr/vi/id，占位、不可交互）
 * 每行：勾选框 + 母语名 + 状态徽标 + 删除按钮（可删且已就绪时）。
 * 下载按钮与进度在后续阶段（download_ocr_model）接入。
 */
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useOcrLanguages } from '@/composables/useOcrLanguages'
import { useToast } from '@/composables/useToast'
import type { OcrLanguageInfo } from '@/types/ocr'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

const { t } = useI18n()
const { push, localizeError } = useToast()
const {
  loading,
  downloading,
  progress,
  failed,
  builtinLanguages,
  downloadableLanguages,
  plannedLanguages,
  isSelected,
  toggle,
  refresh,
  downloadModel,
  deleteModel
} = useOcrLanguages()

onMounted(() => {
  refresh()
})

function badgeKey(lang: OcrLanguageInfo): string {
  if (lang.tier === 'planned') return 'ocr.badgePlanned'
  if (lang.tier === 'downloadable') return 'ocr.badgeDownloadable'
  return 'ocr.badgeBuiltin'
}

function badgeClass(lang: OcrLanguageInfo): string {
  if (lang.tier === 'planned') return 'badge-planned'
  if (lang.tier === 'downloadable') return 'badge-downloadable'
  return 'badge-builtin'
}

/** 显示名：en_opt 走 i18n（「优化」需随界面语言翻译），其余语言用母语名。 */
function displayName(lang: OcrLanguageInfo): string {
  return lang.code === 'en_opt' ? t('ocr.langEnOpt') : lang.native
}

/** 是否具备下载能力（内置随包 + 可下载两类都带下载 URL） */
function canDownload(lang: OcrLanguageInfo): boolean {
  return lang.tier === 'builtin_bundle' || lang.tier === 'downloadable'
}

/** 当前语言是否正在下载 */
function isDownloading(lang: OcrLanguageInfo): boolean {
  return downloading.value === lang.code
}

/** 当前语言的下载进度（百分比） */
function downloadPercent(lang: OcrLanguageInfo): number {
  return progress.value[lang.code]?.percent ?? 0
}

/** 当前语言是否处于「下载失败待重试」 */
function hasFailed(lang: OcrLanguageInfo): boolean {
  return failed.value.includes(lang.code)
}

// 删除二次确认
const confirmOpen = ref(false)
const pendingDelete = ref<OcrLanguageInfo | null>(null)

function requestDelete(lang: OcrLanguageInfo) {
  pendingDelete.value = lang
  confirmOpen.value = true
}

async function onConfirmDelete() {
  if (!pendingDelete.value) return
  try {
    await deleteModel(pendingDelete.value.code)
  } catch (err) {
    push(localizeError(err, t('ocr.verifyFailed')), 'error')
  }
  confirmOpen.value = false
  pendingDelete.value = null
}
</script>

<template>
  <div class="ocr-language-list">
    <p class="ocr-hint">{{ $t('ocr.hint') }}</p>

    <!-- 内置识别 -->
    <div class="ocr-group">
      <p class="ocr-group-title">{{ $t('ocr.builtinGroup') }}</p>
      <ul class="ocr-items">
        <li v-for="lang in builtinLanguages" :key="lang.code" class="ocr-item">
          <label class="ocr-item-main" :class="{ disabled: !lang.available }">
            <input
              type="checkbox"
              :checked="isSelected(lang.code)"
              :disabled="!lang.available"
              @change="toggle(lang.code)"
            />
            <span class="ocr-native">{{ displayName(lang) }}</span>
            <span class="ocr-badge" :class="badgeClass(lang)">{{ $t(badgeKey(lang)) }}</span>
          </label>
          <div v-if="isDownloading(lang)" class="download-progress">
            <div class="progress-track">
              <div class="progress-fill" :style="{ width: downloadPercent(lang) + '%' }"></div>
            </div>
            <span class="progress-text">{{ $t('ocr.downloading', { percent: downloadPercent(lang) }) }}</span>
          </div>
          <button
            v-else-if="!lang.available && canDownload(lang)"
            class="btn-download"
            @click="downloadModel(lang.code)"
          >
            {{ hasFailed(lang) ? $t('ocr.retry') : $t('ocr.download') }}
          </button>
          <button
            v-else-if="lang.deletable && lang.available"
            class="btn-delete"
            :title="$t('ocr.delete')"
            @click="requestDelete(lang)"
          >
            🗑
          </button>
        </li>
      </ul>
    </div>

    <!-- 可下载识别 -->
    <div class="ocr-group">
      <p class="ocr-group-title">{{ $t('ocr.downloadableGroup') }}</p>
      <ul class="ocr-items">
        <li v-for="lang in downloadableLanguages" :key="lang.code" class="ocr-item">
          <label class="ocr-item-main" :class="{ disabled: !lang.available }">
            <input
              type="checkbox"
              :checked="isSelected(lang.code)"
              :disabled="!lang.available"
              @change="toggle(lang.code)"
            />
            <span class="ocr-native">{{ displayName(lang) }}</span>
            <span class="ocr-badge" :class="badgeClass(lang)">
              <span class="cloud">☁</span>{{ $t(badgeKey(lang)) }}
            </span>
          </label>
          <div v-if="isDownloading(lang)" class="download-progress">
            <div class="progress-track">
              <div class="progress-fill" :style="{ width: downloadPercent(lang) + '%' }"></div>
            </div>
            <span class="progress-text">{{ $t('ocr.downloading', { percent: downloadPercent(lang) }) }}</span>
          </div>
          <button
            v-else-if="!lang.available && canDownload(lang)"
            class="btn-download"
            @click="downloadModel(lang.code)"
          >
            {{ hasFailed(lang) ? $t('ocr.retry') : $t('ocr.download') }}
          </button>
          <button
            v-else-if="lang.deletable && lang.available"
            class="btn-delete"
            :title="$t('ocr.delete')"
            @click="requestDelete(lang)"
          >
            🗑
          </button>
        </li>
      </ul>
    </div>

    <!-- 即将推出 -->
    <div class="ocr-group">
      <p class="ocr-group-title">{{ $t('ocr.plannedGroup') }}</p>
      <ul class="ocr-items">
        <li v-for="lang in plannedLanguages" :key="lang.code" class="ocr-item planned">
          <span class="ocr-native muted">{{ displayName(lang) }}</span>
          <span class="ocr-badge badge-planned">{{ $t(badgeKey(lang)) }}</span>
        </li>
      </ul>
    </div>

    <p v-if="loading" class="ocr-loading">{{ $t('common.loading') }}</p>

    <ConfirmDialog
      :visible="confirmOpen"
      :title="$t('ocr.deleteConfirmTitle')"
      :message="$t('ocr.deleteConfirmMessage', { name: pendingDelete?.native || '' })"
      :confirm-text="$t('ocr.delete')"
      danger
      @confirm="onConfirmDelete"
      @cancel="confirmOpen = false"
    />
  </div>
</template>

<style scoped>
.ocr-language-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.ocr-hint {
  margin: 0;
  font-size: 12px;
  color: #666677;
  line-height: 1.5;
}

.ocr-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.ocr-group-title {
  margin: 0;
  font-size: 12px;
  font-weight: 600;
  color: #8888a0;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.ocr-items {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.ocr-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 9px 12px;
  background: rgba(10, 10, 12, 0.4);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 8px;
  transition: border-color 0.15s;
}

.ocr-item:hover {
  border-color: rgba(108, 142, 227, 0.25);
}

.ocr-item.planned {
  opacity: 0.55;
  cursor: not-allowed;
}

.ocr-item-main {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
  cursor: pointer;
}

.ocr-item-main.disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

.ocr-item-main input[type='checkbox'] {
  accent-color: #6c8ee3;
  width: 15px;
  height: 15px;
  cursor: pointer;
}

.ocr-native {
  font-size: 14px;
  color: #f0f0f5;
  font-weight: 500;
}

.ocr-native.muted {
  color: #8888a0;
}

.ocr-badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 999px;
  white-space: nowrap;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.badge-builtin {
  background: rgba(74, 222, 128, 0.12);
  color: #86efac;
  border: 1px solid rgba(74, 222, 128, 0.25);
}

.badge-downloadable {
  background: rgba(108, 142, 227, 0.12);
  color: #93c5fd;
  border: 1px solid rgba(108, 142, 227, 0.3);
}

.badge-planned {
  background: rgba(255, 255, 255, 0.05);
  color: #8888a0;
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.cloud {
  font-size: 12px;
}

.btn-delete {
  flex: 0 0 auto;
  background: none;
  border: none;
  font-size: 13px;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;
  transition: background 0.15s;
}

.btn-delete:hover {
  background: rgba(248, 113, 113, 0.15);
}

.btn-download {
  flex: 0 0 auto;
  padding: 5px 12px;
  font-size: 12px;
  font-weight: 500;
  color: #93c5fd;
  background: linear-gradient(135deg, rgba(108, 142, 227, 0.18), rgba(74, 111, 207, 0.12));
  border: 1px solid rgba(108, 142, 227, 0.35);
  border-radius: 6px;
  cursor: pointer;
  white-space: nowrap;
  transition: filter 0.15s, transform 0.1s;
}

.btn-download:hover {
  filter: brightness(1.15);
}

.download-progress {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 120px;
}

.progress-track {
  flex: 1;
  height: 6px;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 999px;
  overflow: hidden;
  min-width: 60px;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #6c8ee3, #4a6fcf);
  border-radius: 999px;
  transition: width 0.2s ease;
}

.progress-text {
  font-size: 11px;
  color: #93c5fd;
  white-space: nowrap;
}

.ocr-loading {
  margin: 0;
  font-size: 12px;
  color: #6c8ee3;
}
</style>
