<script setup lang="ts">
/**
 * SmartFolderBar — 智能文件夹 pill 列表（P1-3 升级）。
 *
 * 第三轮修复：
 *   - pill 显示查询条件 tooltip（hover 看完整条件）；
 *   - 计数显示去黑话（不再 `(Num: 12 )`，改 `12` 徽标）；
 *   - `?` 加 tooltip 解释「需点击执行才知道」；
 *   - 删除按钮独立 hover 区域，不再与 pill 整体混在一起（减少误点）；
 *   - 删除走二次确认（由 useSmartFolders.removeSmartFolder 内部 confirm）；
 *   - 加刷新计数按钮（纯语义文件夹可手动拿真实数字）。
 */
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useToast } from '@/composables/useToast'
import { exportJsonContent } from '@/utils/exporters'
import type { SmartFolder, SmartFolderImportResult } from '@/types/search'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

const props = defineProps<{
  smartFolders: SmartFolder[]
  /** M0'''：正在刷新计数的文件夹 id 集合（由父组件真实完成时移除） */
  refreshing: Set<number>
}>()

const emit = defineEmits<{
  (e: 'apply', folder: SmartFolder): void
  (e: 'remove', id: number, name?: string): void
  (e: 'refreshCount', id: number): void
  (e: 'edit', folder: SmartFolder): void
  (e: 'changed'): void
}>()

const { t } = useI18n()

// M0'''：超过 3 秒仍未完成的刷新，提示从「刷新中」切换为「仍在进行…」，
// 但**绝不伪造完成**——完成态只能由父组件真实刷新完成后移除。
const longRunning = ref<Set<number>>(new Set())

// P0-4：删除二次确认（应用内 ConfirmDialog，替代 window.confirm）
const confirmOpen = ref(false)
const pendingDelete = ref<SmartFolder | null>(null)
function onRequestDelete(sf: SmartFolder) {
  pendingDelete.value = sf
  confirmOpen.value = true
}
function onConfirmDelete() {
  if (pendingDelete.value) emit('remove', pendingDelete.value.id, pendingDelete.value.name)
  confirmOpen.value = false
  pendingDelete.value = null
}

function buildTooltip(sf: SmartFolder): string {
  const parts: string[] = [t('smartFolder.query', { query: sf.query_text || t('smartFolder.empty') })]
  if (sf.needs_frontend_eval) {
    // 第八轮：条件行文件夹的匹配数由前端引擎算，列表里给不出真值 —— 说清楚，不假装。
    parts.push(t('smartFolder.condCountHint'))
    return parts.join('\n')
  }
  const channels: string[] = []
  if (sf.use_vector) channels.push(t('smartFolder.visualSemantic'))
  if (sf.use_ocr) channels.push(t('smartFolder.ocr'))
  if (sf.use_note) channels.push(t('smartFolder.note'))
  if (sf.use_filename) channels.push(t('smartFolder.filename'))
  parts.push(t('smartFolder.channels', { channels: channels.join(' / ') || t('smartFolder.none') }))
  if (sf.match_count === 0 && sf.use_vector && !(sf.use_ocr || sf.use_note || sf.use_filename)) {
    parts.push(t('smartFolder.countHint'))
  }
  return parts.join('\n')
}

/** 计数徽标：该显示数字还是「?」。第八轮新增 needs_frontend_eval 一档。 */
function countUnknown(sf: SmartFolder): boolean {
  if (sf.needs_frontend_eval) return true
  return sf.match_count === 0 && sf.use_vector && !(sf.use_ocr || sf.use_note || sf.use_filename)
}

function onRefresh(e: MouseEvent, id: number) {
  e.stopPropagation()
  // 新一轮刷新：清掉上一轮残留的「仍在进行」标记
  longRunning.value.delete(id)
  emit('refreshCount', id)
  // M0'''：完成态由父组件真实刷新完成后移除（refreshing 集合）。
  // 这里只在 3 秒后仍未完成时，把提示切换为「仍在进行…」，不伪造完成。
  setTimeout(() => {
    if (props.refreshing.has(id)) longRunning.value.add(id)
  }, 3000)
}

// M2''' 智能文件夹导出（补上「出」）：导出**条件定义**（去 id / match_count 噪音），
// 走统一导出通道（exportJsonContent），尊重用户配置的 exportPath。
function exportSmartFolders() {
  const stamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)
  const clean = props.smartFolders.map((sf) => ({
    name: sf.name,
    query_text: sf.query_text,
    use_vector: sf.use_vector,
    use_ocr: sf.use_ocr,
    use_note: sf.use_note,
    use_filename: sf.use_filename,
    // 第八轮：条件行定义一起导出，否则导入回来会退化成旧 7 字段（丢掉排除/阈值/筛选）。
    criteria_json: sf.criteria_json ?? null
  }))
  exportJsonContent({ exported_at: new Date().toISOString(), smart_folders: clean }, `framescout_smart_folders_${stamp}.json`)
}

// M2''' 智能文件夹导入（补上「入」）：读取导出的 JSON，逐个写回 smart_folders 表。
async function importSmartFolders() {
  const { push } = useToast()
  const selected = await open({
    multiple: false,
    filters: [{ name: 'JSON', extensions: ['json'] }]
  })
  if (!selected) return
  try {
    const raw = await (await fetch(convertToFileUrl(selected as string))).text()
    const parsed = JSON.parse(raw)
    const list = (parsed.smart_folders ?? parsed) as SmartFolder[]
    if (!Array.isArray(list)) {
      push(t('smartFolder.importInvalid'), 'error')
      return
    }
    // 第八轮：后端现在按名字去重（撞名加序号后缀，绝不覆盖已有定义），
    // 并如实回报「导入 N 条，其中 M 条因重名改了名」。
    const res = await invoke<SmartFolderImportResult>('import_smart_folders', { folders: list })
    push(t('smartFolder.imported', { count: res.imported }), 'success')
    if (res.renamed > 0) {
      push(t('smartFolder.importRenamed', { count: res.renamed }), 'warning')
    }
    emit('changed')
  } catch (err) {
    push(t('smartFolder.importFailed', { err }), 'error')
  }
}

/** 把本地文件路径转成可 fetch 的 URL（Windows 盘符路径）。 */
function convertToFileUrl(p: string): string {
  return 'file:///' + p.replace(/\\/g, '/')
}
</script>

<template>
  <div v-if="smartFolders.length > 0" class="smart-folders-bar">
    <span class="smart-folder-label">{{ $t('smartFolder.title') }}</span>
    <button class="sf-export" :title="$t('smartFolder.export')" @click="exportSmartFolders">⬇️</button>
    <button class="sf-export" :title="$t('smartFolder.import')" @click="importSmartFolders">⬆️</button>

    <div
      v-for="sf in smartFolders"
      :key="sf.id"
      class="smart-folder-pill"
      :title="buildTooltip(sf)"
      @click="emit('apply', sf)"
    >
      <span class="sf-name">📁 {{ sf.name }}</span>
      <span
        class="sf-count-badge"
        :class="{ 'sf-count-unknown': countUnknown(sf) }"
        :title="countUnknown(sf) ? (sf.needs_frontend_eval ? $t('smartFolder.condCountHint') : $t('smartFolder.semanticOnly')) : $t('smartFolder.matchCount', { count: sf.match_count })"
      >
        {{ countUnknown(sf) ? '?' : sf.match_count }}
      </span>
      <button
        class="sf-refresh"
        :disabled="props.refreshing.has(sf.id)"
        @click="onRefresh($event, sf.id)"
        :title="longRunning.has(sf.id) ? $t('smartFolder.stillRunning') : (props.refreshing.has(sf.id) ? $t('smartFolder.refreshing') : $t('smartFolder.refreshCount'))"
      >↻</button>
      <button
        class="sf-edit"
        @click.stop="emit('edit', sf)"
        :title="$t('smartFolder.edit')"
      >✏️</button>
      <button
        class="sf-delete"
        @click.stop="onRequestDelete(sf)"
        :title="$t('smartFolder.delete')"
      >✕</button>
    </div>

    <!-- P0-4：智能文件夹删除二次确认（应用内，替代 window.confirm） -->
    <ConfirmDialog
      :visible="confirmOpen"
      :title="$t('smartFolder.deleteConfirmTitle')"
      :message="$t('smartFolder.confirmDelete', { name: pendingDelete?.name || '#' + (pendingDelete?.id ?? '') })"
      :confirm-text="$t('smartFolder.delete')"
      danger
      @confirm="onConfirmDelete"
      @cancel="confirmOpen = false"
    />
  </div>
</template>

<style scoped>
.smart-folders-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  margin-bottom: 20px;
  flex-wrap: wrap;
}

.smart-folder-label {
  font-size: 12px;
  color: #8888a0;
  font-weight: bold;
}

.smart-folder-pill {
  display: flex;
  align-items: center;
  gap: 6px;
  background: linear-gradient(135deg, rgba(20, 20, 31, 0.95), rgba(28, 28, 46, 0.95));
  border: 1px solid rgba(108, 142, 227, 0.2);
  padding: 5px 8px 5px 12px;
  border-radius: 20px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.18s;
  white-space: nowrap;
}
.smart-folder-pill:hover {
  border-color: rgba(108, 142, 227, 0.5);
  background: linear-gradient(135deg, rgba(28, 28, 46, 0.95), rgba(40, 40, 60, 0.95));
  box-shadow: 0 2px 12px rgba(108, 142, 227, 0.12);
}

.sf-name {
  color: #f0f0f5;
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sf-count-badge {
  color: #6c8ee3;
  font-size: 11px;
  font-weight: 600;
  min-width: 16px;
  text-align: center;
}
.sf-count-badge.sf-count-unknown {
  color: #8888a0;
}

.sf-refresh,
.sf-edit,
.sf-delete {
  background: none;
  border: none;
  font-size: 11px;
  cursor: pointer;
  padding: 0 4px;
  border-radius: 50%;
  transition: all 0.15s;
  line-height: 1;
}

.sf-export {
  background: none;
  border: none;
  font-size: 12px;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 6px;
  color: #8888a0;
  transition: color 0.15s, background 0.15s;
}
.sf-export:hover {
  color: #6c8ee3;
  background: rgba(108, 142, 227, 0.15);
}

.sf-edit {
  color: #666677;
}
.sf-edit:hover {
  color: #6c8ee3;
  background: rgba(108, 142, 227, 0.15);
}

.sf-refresh {
  color: #666677;
}
.sf-refresh:hover:not(:disabled) {
  color: #6c8ee3;
  background: rgba(108, 142, 227, 0.15);
}
.sf-refresh:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.sf-delete {
  color: #666677;
}
.sf-delete:hover {
  color: #f87171;
  background: rgba(248, 113, 113, 0.15);
}
</style>
