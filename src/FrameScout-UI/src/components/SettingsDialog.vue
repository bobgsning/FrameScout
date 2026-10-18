<script setup lang="ts">
/**
 * SettingsDialog — 设置页骨架（P1-5 / 第三轮「无设置 / 偏好页面」）。
 *
 * 第一版只收口最痛的几项：
 *   - 素材库路径（不再每次启动被清掉）
 *   - OCR 语言（带常用选项下拉）
 *   - 每页条数
 *   - 关于（版本号、引擎状态）
 * 后续可在此面板内扩展（融合权重、备份、深色模式等）。
 */
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { usePreferences } from '@/composables/usePreferences'
import { useToast } from '@/composables/useToast'
import { invoke } from '@tauri-apps/api/core'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { open } from '@tauri-apps/plugin-dialog'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import OcrLanguageList from '@/components/OcrLanguageList.vue'
import type { CleanupSettings, BackupResult, StartupCleanupReport } from '@/types/search'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const { folderPath, pageSize, language, exportPath, searchHistoryLimit, relevanceSensitivity, lowScoreFoldEnabled } =
  usePreferences()
const { push } = useToast()
const { t } = useI18n()

// ---- M0''' 自动清理策略（事件保留天数 / 备份份数 / 备份天数） ----
// `-1` = 「永不 / 不限」。落库到后端 app_settings（启动期裁剪读取）。
const cleanup = ref<CleanupSettings>({
  event_retention_days: 90,
  backup_keep_count: 3,
  backup_max_age_days: 30
})

async function loadCleanupSettings() {
  try {
    cleanup.value = await invoke<CleanupSettings>('get_cleanup_settings')
  } catch (err) {
    push(t('settings.cleanupLoadFailed', { err }), 'error')
  }
}

async function saveCleanupSettings() {
  try {
    await invoke('set_cleanup_settings', { settings: cleanup.value })
  } catch (err) {
    push(t('settings.cleanupSaveFailed', { err }), 'error')
  }
}

// ---- M1'''' 清理报告可回看：上次启动清理了什么（不可逆删除有账可查） ----
const lastCleanup = ref<StartupCleanupReport | null>(null)
async function loadLastCleanup() {
  try {
    lastCleanup.value = await invoke<StartupCleanupReport>('get_startup_cleanup_report')
  } catch {
    lastCleanup.value = null
  }
}
function hasLastCleanup(): boolean {
  const r = lastCleanup.value
  if (!r) return false
  return r.pruned_events > 0 || r.pruned_batches > 0 || r.pruned_reindex_backups > 0 || r.cleaned_residual_rows > 0
}

/** 第八轮：把 UTC ISO 时间显示成本地可读时刻（否则用户看到的是「2026-10-14T02:11:03Z」）。 */
function formatCleanupTime(iso: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}

// 导出目录：用户可选一个默认导出位置并持久化
async function selectExportDir() {
  const selected = await open({ directory: true })
  if (selected) {
    exportPath.value = selected as string
  }
}
function clearExportPath() {
  exportPath.value = ''
}

// P2-5 / 债单 A4：备份列表（list_backups 此前零调用方，备份「能建不能列」）
interface BackupInfo {
  name: string
  path: string
  size_bytes: number
  modified_ts: number
}

const backups = ref<BackupInfo[]>([])

async function listBackups() {
  try {
    backups.value = await invoke<BackupInfo[]>('list_backups')
  } catch (err) {
    backups.value = []
    // P1-10：备份列表加载失败此前静默置空（与「本来就没备份」不可区分）；现给可见反馈
    push(t('settings.backupListFailed', { err }), 'error')
  }
}

function formatBackupSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

function formatBackupTime(ts: number): string {
  if (!ts) return '—'
  return new Date(ts * 1000).toLocaleString()
}

// 打开时加载备份列表 + 自动清理策略 + 上次启动清理报告
watch(
  () => props.visible,
  (v) => {
    if (v) {
      listBackups()
      loadCleanupSettings()
      loadLastCleanup()
    }
  }
)

// P2-5：备份（M0'''：返回被轮转删除的旧备份文件名，Toast 告知）
async function onBackup() {
  try {
    const result = await invoke<BackupResult>('backup_database')
    if (result.removed.length > 0) {
      push(
        t('settings.backupCreatedWithRotation', { path: result.path, removed: result.removed.join(', ') }),
        'success',
        6000
      )
    } else {
      push(t('settings.backupCreated', { path: result.path }), 'success', 5000)
    }
    await listBackups()
  } catch (err) {
    push(t('settings.backupFailed', { err }), 'error')
  }
}

async function onVerifyIntegrity() {
  try {
    const result = await invoke<string>('verify_integrity')
    if (result === 'ok') {
      push(t('settings.integrityOk'), 'success')
    } else {
      push(t('settings.integrityResult', { result }), 'warning', 8000)
    }
  } catch (err) {
    push(t('settings.verifyFailed', { err }), 'error')
  }
}

// P2-8：删除备份 + 打开所在目录（此前备份「能建不能删」）
const confirmOpen = ref(false)
const pendingDelete = ref<BackupInfo | null>(null)
function onRequestDeleteBackup(b: BackupInfo) {
  pendingDelete.value = b
  confirmOpen.value = true
}
async function onConfirmDeleteBackup() {
  if (!pendingDelete.value) return
  try {
    await invoke('delete_backup', { path: pendingDelete.value.path })
    confirmOpen.value = false
    pendingDelete.value = null
    push(t('settings.backupDeleted'), 'success')
    await listBackups()
  } catch (err) {
    push(t('settings.backupDeleteFailed', { err }), 'error')
  }
}
async function openBackupDir(b: BackupInfo) {
  try {
    await revealItemInDir(b.path)
  } catch (err) {
    push(t('file.revealFailed', { err }), 'error')
  }
}

// M2'''：备份恢复（pending restore，下次启动生效）
// M1'''' P0-2：确认前先预览「当前 N 条 → 备份 M 条」，确认后自动备份当前库再写标记。
interface RestorePreview {
  current_frames: number
  backup_frames: number
}
const restoreConfirmOpen = ref(false)
const pendingRestore = ref<BackupInfo | null>(null)
const restorePreview = ref<RestorePreview | null>(null)
const restorePreviewBusy = ref(false)

async function onRequestRestore(b: BackupInfo) {
  pendingRestore.value = b
  restorePreview.value = null
  restorePreviewBusy.value = true
  try {
    restorePreview.value = await invoke<RestorePreview>('preview_restore', { path: b.path })
  } catch {
    restorePreview.value = null
  } finally {
    restorePreviewBusy.value = false
  }
  restoreConfirmOpen.value = true
}

/** 生成「会丢失/增加多少」的文案 */
function restoreDeltaText(): string {
  const p = restorePreview.value
  if (!p) return ''
  const delta = p.current_frames - p.backup_frames
  if (delta > 0) return t('settings.restoreWillLose', { n: delta })
  if (delta < 0) return t('settings.restoreWillGain', { n: -delta })
  return t('settings.restoreSameCount')
}

/** 确认框完整文案：当前 N 条 → 备份 M 条 + 将丢失/增加 X 条。 */
function restoreBackupMessageText(): string {
  const name = pendingRestore.value?.name || ''
  const p = restorePreview.value
  if (!p) return t('settings.restoreBackupMessage', { name })
  const compare = t('settings.restoreFramesCompare', { current: p.current_frames, backup: p.backup_frames })
  return `${t('settings.restoreBackupMessage', { name })} ${compare}${restoreDeltaText() ? ' · ' + restoreDeltaText() : ''}`
}

async function onConfirmRestore() {
  if (!pendingRestore.value) return
  try {
    const res = await invoke<{ auto_backup_path: string; current_frames: number; backup_frames: number }>(
      'restore_backup',
      { path: pendingRestore.value.path }
    )
    restoreConfirmOpen.value = false
    pendingRestore.value = null
    restorePreview.value = null
    push(t('settings.restorePendingDetail', { current: res.current_frames, backup: res.backup_frames }), 'success', 8000)
  } catch (err) {
    push(t('settings.restoreFailed', { err }), 'error')
  }
}

// 界面语言选项（国际化扩展：9 种界面语言，母语名显示）
const languageOptions = [
  { value: 'en', labelKey: 'settings.languageEn' },
  { value: 'zh-CN', labelKey: 'settings.languageZhCN' },
  { value: 'zh-TW', labelKey: 'settings.languageZhTW' },
  { value: 'ja', labelKey: 'settings.languageJa' },
  { value: 'ru', labelKey: 'settings.languageRu' },
  { value: 'ko', labelKey: 'settings.languageKo' },
  { value: 'de', labelKey: 'settings.languageDe' },
  { value: 'fr', labelKey: 'settings.languageFr' },
  { value: 'es', labelKey: 'settings.languageEs' },
]

// P1-9：与搜索栏统一取值集合（三处收敛为一套）
// 第八轮：补 100 —— 大库翻页太碎，100 张/页是设计文档点名要的档位。
const pageSizeOptions = [4, 8, 16, 32, 50, 100]

// 应用版本号（从 package.json 读取，Vite 注入；兜底值仅防 define 未生效）
const appVersion = __APP_VERSION__ || '3.3.0'

function clearFolderPath() {
  folderPath.value = ''
}
</script>

<template>
  <Teleport to="body">
    <div v-if="props.visible" class="settings-overlay" @click.self="emit('close')">
      <div class="settings-dialog">
        <div class="settings-header">
          <h2 class="settings-title">{{ $t('settings.title') }}</h2>
          <button class="settings-close" @click="emit('close')">✕</button>
        </div>

        <div class="settings-body">
          <!-- 语言 -->
          <div class="setting-group">
            <label class="setting-label">🌐 {{ $t('settings.language') }}</label>
            <p class="setting-hint">{{ $t('settings.languageHint') }}</p>
            <select v-model="language" class="setting-select">
              <option v-for="opt in languageOptions" :key="opt.value" :value="opt.value">
                {{ $t(opt.labelKey) }}
              </option>
            </select>
          </div>

          <!-- 素材库路径 -->
          <div class="setting-group">
            <label class="setting-label">📂 {{ $t('settings.folderPath') }}</label>
            <p class="setting-hint">{{ $t('settings.folderPathHint') }}</p>
            <div class="folder-input-row">
              <input
                v-model="folderPath"
                type="text"
                class="folder-input"
                :placeholder="$t('settings.folderPathPlaceholder')"
              />
              <button class="btn-clear" @click="clearFolderPath" :disabled="!folderPath">{{ $t('settings.clear') }}</button>
            </div>
          </div>

          <!-- OCR 识别语言（多选列表，三档分组） -->
          <div class="setting-group">
            <label class="setting-label">🔤 {{ $t('ocr.title') }}</label>
            <OcrLanguageList />
          </div>

          <!-- 每页条数 -->
          <div class="setting-group">
            <label class="setting-label">📄 {{ $t('settings.pageSize') }}</label>
            <p class="setting-hint">{{ $t('settings.pageSizeHint') }}</p>
            <select v-model="pageSize" class="setting-select">
              <option v-for="n in pageSizeOptions" :key="n" :value="n">{{ $t('settings.perPage', { n }) }}</option>
            </select>
          </div>

          <!-- M0'''：搜索历史保留条数（此前静默截到 20，现可设「不限」） -->
          <div class="setting-group">
            <label class="setting-label">🕘 {{ $t('settings.historyLimit') }}</label>
            <p class="setting-hint">{{ $t('settings.historyLimitHint') }}</p>
            <select v-model="searchHistoryLimit" class="setting-select">
              <option :value="10">{{ $t('settings.historyEntries', { n: 10 }) }}</option>
              <option :value="20">{{ $t('settings.historyEntries', { n: 20 }) }}</option>
              <option :value="50">{{ $t('settings.historyEntries', { n: 50 }) }}</option>
              <option :value="100">{{ $t('settings.historyEntries', { n: 100 }) }}</option>
              <option :value="-1">{{ $t('settings.unlimited') }}</option>
            </select>
          </div>

          <!-- M0'''：相关度灵敏度 + 低分折叠开关 -->
          <div class="setting-group">
            <label class="setting-label">🎚️ {{ $t('settings.relevanceSensitivity') }}</label>
            <p class="setting-hint">{{ $t('settings.relevanceSensitivityHint') }}</p>
            <select v-model="relevanceSensitivity" class="setting-select">
              <option value="loose">{{ $t('settings.sensitivityLoose') }}</option>
              <option value="standard">{{ $t('settings.sensitivityStandard') }}</option>
              <option value="strict">{{ $t('settings.sensitivityStrict') }}</option>
            </select>
          </div>

          <!-- M0'''：低分自动折叠开关（「不折叠」是一等选项） -->
          <div class="setting-group">
            <label class="setting-toggle">
              <input v-model="lowScoreFoldEnabled" type="checkbox" class="setting-checkbox" />
              {{ $t('settings.lowScoreFold') }}
            </label>
            <p class="setting-hint">{{ $t('settings.lowScoreFoldHint') }}</p>
          </div>

          <!-- 导出目录（Batch 31：用户可设默认导出位置） -->
          <div class="setting-group">
            <label class="setting-label">📤 {{ $t('settings.exportDir') }}</label>
            <p class="setting-hint">{{ $t('settings.exportDirHint') }}</p>
            <div class="folder-input-row">
              <input
                v-model="exportPath"
                type="text"
                class="folder-input"
                :placeholder="$t('settings.exportDirPlaceholder')"
              />
              <button class="btn-export-browse" @click="selectExportDir">{{ $t('settings.browse') }}</button>
              <button class="btn-clear" @click="clearExportPath" :disabled="!exportPath">{{ $t('settings.clear') }}</button>
            </div>
          </div>

          <!-- M0'''：自动清理策略（给开关，支持「永不 / 不限」） -->
          <div class="setting-group">
            <label class="setting-label">🧹 {{ $t('settings.cleanupTitle') }}</label>
            <p class="setting-hint">{{ $t('settings.cleanupHint') }}</p>
            <div class="cleanup-grid">
              <div class="cleanup-field">
                <span class="cleanup-label">{{ $t('settings.eventRetention') }}</span>
                <select v-model="cleanup.event_retention_days" class="setting-select" @change="saveCleanupSettings">
                  <option :value="30">{{ $t('settings.days', { n: 30 }) }}</option>
                  <option :value="90">{{ $t('settings.days', { n: 90 }) }}</option>
                  <option :value="180">{{ $t('settings.days', { n: 180 }) }}</option>
                  <option :value="365">{{ $t('settings.days', { n: 365 }) }}</option>
                  <option :value="-1">{{ $t('settings.never') }}</option>
                </select>
              </div>
              <div class="cleanup-field">
                <span class="cleanup-label">{{ $t('settings.backupKeepCount') }}</span>
                <select v-model="cleanup.backup_keep_count" class="setting-select" @change="saveCleanupSettings">
                  <option :value="1">{{ $t('settings.backupCopies', { n: 1 }) }}</option>
                  <option :value="2">{{ $t('settings.backupCopies', { n: 2 }) }}</option>
                  <option :value="3">{{ $t('settings.backupCopies', { n: 3 }) }}</option>
                  <option :value="5">{{ $t('settings.backupCopies', { n: 5 }) }}</option>
                  <option :value="10">{{ $t('settings.backupCopies', { n: 10 }) }}</option>
                  <option :value="-1">{{ $t('settings.unlimited') }}</option>
                </select>
              </div>
              <div class="cleanup-field">
                <span class="cleanup-label">{{ $t('settings.backupMaxAge') }}</span>
                <select v-model="cleanup.backup_max_age_days" class="setting-select" @change="saveCleanupSettings">
                  <option :value="7">{{ $t('settings.days', { n: 7 }) }}</option>
                  <option :value="30">{{ $t('settings.days', { n: 30 }) }}</option>
                  <option :value="90">{{ $t('settings.days', { n: 90 }) }}</option>
                  <option :value="365">{{ $t('settings.days', { n: 365 }) }}</option>
                  <option :value="-1">{{ $t('settings.unlimited') }}</option>
                </select>
              </div>
            </div>

            <!-- M1''''：上次启动清理（不可逆删除有账可查，不再只有一次性 Toast） -->
            <div v-if="hasLastCleanup()" class="last-cleanup">
              <span class="last-cleanup-title">{{ $t('settings.lastCleanup') }}</span>
              <span v-if="lastCleanup && lastCleanup.cleaned_at" class="last-cleanup-time">
                {{ $t('settings.cleanupAt', { at: formatCleanupTime(lastCleanup.cleaned_at) }) }}
              </span>
              <span v-if="lastCleanup!.pruned_events > 0">{{ $t('settings.cleanupEvents', { n: lastCleanup!.pruned_events }) }}</span>
              <span v-if="lastCleanup!.pruned_batches > 0">{{ $t('settings.cleanupBatches', { n: lastCleanup!.pruned_batches }) }}</span>
              <span v-if="lastCleanup!.pruned_reindex_backups > 0">{{ $t('settings.cleanupBackups', { n: lastCleanup!.pruned_reindex_backups }) }}</span>
              <span v-if="lastCleanup!.cleaned_residual_rows > 0">{{ $t('settings.cleanupRows', { n: lastCleanup!.cleaned_residual_rows }) }}</span>
            </div>
            <p v-else class="last-cleanup-empty">{{ $t('settings.lastCleanupNone') }}</p>
          </div>

          <!-- 备份与维护（P2-5） -->
          <div class="setting-group">
            <label class="setting-label">💾 {{ $t('settings.backup') }}</label>
            <p class="setting-hint">{{ $t('settings.backupHint') }}</p>
            <div class="backup-row">
              <button class="btn-backup" @click="onBackup">{{ $t('settings.backupNow') }}</button>
              <button class="btn-verify" @click="onVerifyIntegrity">{{ $t('settings.verifyIntegrity') }}</button>
            </div>

            <!-- 已有备份列表（债单 A4：能建也能列） -->
            <div v-if="backups.length" class="backup-list">
              <p class="backup-list-title">{{ $t('settings.existingBackups', { count: backups.length }) }}</p>
              <ul class="backup-items">
                <li v-for="b in backups" :key="b.name" class="backup-item">
                  <span class="backup-name" :title="b.path">{{ b.name }}</span>
                  <span class="backup-meta">{{ formatBackupSize(b.size_bytes) }} · {{ formatBackupTime(b.modified_ts) }}</span>
                  <span class="backup-actions">
                    <button class="btn-icon" :title="$t('settings.restoreBackup')" @click="onRequestRestore(b)">🔄</button>
                    <button class="btn-icon" :title="$t('settings.openBackupDir')" @click="openBackupDir(b)">📂</button>
                    <button class="btn-icon btn-icon-danger" :title="$t('settings.deleteBackup')" @click="onRequestDeleteBackup(b)">🗑</button>
                  </span>
                </li>
              </ul>
            </div>
            <p class="backup-warning">{{ $t('settings.backupWarning') }}</p>
          </div>

          <!-- 关于 -->
          <div class="setting-group">
            <label class="setting-label">ℹ️ {{ $t('settings.about') }}</label>
            <div class="about-info">
              <p>{{ $t('settings.aboutDesc') }}</p>
              <p class="about-version">{{ $t('settings.version', { version: appVersion }) }}</p>
              <p class="about-tagline">{{ $t('settings.tagline') }}</p>
            </div>
          </div>
        </div>

        <div class="settings-footer">
          <button class="btn-done" @click="emit('close')">{{ $t('common.done') }}</button>
        </div>

        <!-- P2-8：备份删除二次确认 -->
        <ConfirmDialog
          :visible="confirmOpen"
          :title="$t('settings.deleteBackupTitle')"
          :message="$t('settings.deleteBackupMessage', { name: pendingDelete?.name || '' })"
          :confirm-text="$t('settings.deleteBackup')"
          danger
          @confirm="onConfirmDeleteBackup"
          @cancel="confirmOpen = false"
        />

        <!-- M2'''：备份恢复二次确认（下次启动生效，含帧数对比） -->
        <ConfirmDialog
          :visible="restoreConfirmOpen"
          :title="$t('settings.restoreBackupTitle')"
          :message="restoreBackupMessageText()"
          :confirm-text="$t('settings.restoreBackup')"
          danger
          @confirm="onConfirmRestore"
          @cancel="restoreConfirmOpen = false"
        />
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.settings-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
}

.settings-dialog {
  width: 560px;
  max-width: 90vw;
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  background: linear-gradient(180deg, rgba(28, 28, 46, 0.98), rgba(20, 20, 30, 0.98));
  border: 1px solid rgba(108, 142, 227, 0.3);
  border-radius: 16px;
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
  overflow: hidden;
}

.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.settings-title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: #f0f0f5;
}

.settings-close {
  background: none;
  border: none;
  color: #8888a0;
  font-size: 18px;
  cursor: pointer;
  padding: 4px 10px;
  border-radius: 6px;
  transition: background 0.15s, color 0.15s;
}

.settings-close:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #f0f0f5;
}

.settings-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.setting-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.setting-label {
  font-size: 14px;
  font-weight: 500;
  color: #b0b0c0;
}

.setting-hint {
  margin: 0;
  font-size: 12px;
  color: #666677;
  line-height: 1.5;
}

.folder-input-row {
  display: flex;
  gap: 8px;
}

.folder-input,
.setting-select {
  flex: 1;
  padding: 10px 14px;
  background: rgba(10, 10, 12, 0.6);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  color: #f0f0f5;
  font-size: 13px;
  font-family: 'Noto Sans', system-ui, sans-serif;
  outline: none;
  transition: border-color 0.15s;
}

.folder-input:focus,
.setting-select:focus {
  border-color: rgba(108, 142, 227, 0.5);
}

.setting-select {
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12'%3E%3Cpath fill='%238888a0' d='M6 8L2 4h8z'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 12px center;
  padding-right: 32px;
}

.btn-clear {
  padding: 10px 16px;
  background: rgba(248, 113, 113, 0.12);
  border: 1px solid rgba(248, 113, 113, 0.25);
  border-radius: 8px;
  color: #f87171;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s;
  white-space: nowrap;
}

.btn-export-browse {
  padding: 10px 16px;
  background: rgba(108, 142, 227, 0.14);
  border: 1px solid rgba(108, 142, 227, 0.3);
  border-radius: 8px;
  color: #93c5fd;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s;
  white-space: nowrap;
}
.btn-export-browse:hover {
  background: rgba(108, 142, 227, 0.24);
}

.btn-clear:hover:not(:disabled) {
  background: rgba(248, 113, 113, 0.2);
}

.btn-clear:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.about-info {
  padding: 16px;
  background: rgba(10, 10, 12, 0.4);
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.05);
}

.about-info p {
  margin: 0 0 4px 0;
  font-size: 13px;
  color: #b0b0c0;
}

.about-version {
  color: #6c8ee3 !important;
  font-weight: 500;
}

.about-tagline {
  color: #666677 !important;
  font-size: 12px !important;
  margin-top: 8px !important;
}

.settings-footer {
  padding: 16px 24px;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  display: flex;
  justify-content: flex-end;
}

.btn-done {
  padding: 10px 24px;
  background: linear-gradient(135deg, #6c8ee3, #4a6fcf);
  border: none;
  border-radius: 8px;
  color: #fff;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: filter 0.15s, transform 0.1s;
}

.btn-done:hover {
  filter: brightness(1.1);
}

.btn-done:active {
  transform: scale(0.98);
}

.backup-row {
  display: flex;
  gap: 10px;
}

.btn-backup,
.btn-verify {
  padding: 10px 20px;
  border: 1px solid;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s;
  font-family: 'Noto Sans', system-ui, sans-serif;
}

.btn-backup {
  background: linear-gradient(135deg, rgba(108, 142, 227, 0.15), rgba(74, 111, 207, 0.1));
  border-color: rgba(108, 142, 227, 0.35);
  color: #93c5fd;
}

.btn-backup:hover {
  background: linear-gradient(135deg, rgba(108, 142, 227, 0.25), rgba(74, 111, 207, 0.15));
}

.btn-verify {
  background: rgba(74, 222, 128, 0.1);
  border-color: rgba(74, 222, 128, 0.25);
  color: #86efac;
}

.btn-verify:hover {
  background: rgba(74, 222, 128, 0.18);
}

.backup-list {
  margin-top: 4px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.backup-list-title {
  margin: 0;
  font-size: 12px;
  color: #8888a0;
}

.backup-items {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 140px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.backup-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 7px 10px;
  background: rgba(10, 10, 12, 0.4);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 6px;
}

.backup-name {
  font-size: 12px;
  color: #b0b0c0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.backup-meta {
  font-size: 11px;
  color: #666677;
  flex: 0 0 auto;
}

.backup-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: 0 0 auto;
}
.btn-icon {
  background: none;
  border: none;
  font-size: 13px;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;
  transition: background 0.15s;
}
.btn-icon:hover {
  background: rgba(108, 142, 227, 0.15);
}
.btn-icon-danger:hover {
  background: rgba(248, 113, 113, 0.15);
}

.backup-warning {
  margin: 2px 0 0;
  font-size: 12px;
  color: #ffaa33;
  line-height: 1.5;
}

.cleanup-grid {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.cleanup-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.last-cleanup {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 10px 12px;
  background: rgba(255, 170, 51, 0.06);
  border: 1px dashed rgba(255, 170, 51, 0.25);
  border-radius: 6px;
  font-size: 12px;
  color: #ffaa33;
}
.last-cleanup-title {
  font-weight: 600;
  color: #e8a86a;
}
.last-cleanup-empty {
  margin: 0;
  font-size: 12px;
  color: #666677;
}

.cleanup-label {
  font-size: 12px;
  color: #8888a0;
}

.setting-toggle {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 500;
  color: #b0b0c0;
  cursor: pointer;
  user-select: none;
}

.setting-checkbox {
  width: 16px;
  height: 16px;
  accent-color: #6c8ee3;
  cursor: pointer;
}
</style>
