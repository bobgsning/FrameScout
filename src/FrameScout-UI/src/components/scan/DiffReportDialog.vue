<script setup lang="ts">
/**
 * 差异报告对话框 —— 三层交互：总览 → 明细 → 行动。
 *
 * 系统只负责「感知」：把磁盘与库之间的差异摆出来。
 * 索引 / 标记失效 / 忽略 全部由用户勾选后明确触发；
 * 「忽略」刻意不落库 —— 它只作用于本次批次，下次扫描仍会再见到该文件，
 * 这样 observed_* 始终是纯客观感知信号，不会被临时决策污染。
 */
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { DiffItem, DiffKind, DiffReport } from '@/types/search'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

const props = defineProps<{
  open: boolean
  report: DiffReport | null
  busy: boolean
  message: string
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'index', paths: string[]): void
  (e: 'mark-dead', paths: string[]): void
  (e: 'ignore', paths: string[]): void
}>()

const { t } = useI18n()

const GROUPS = computed<{ kind: DiffKind; title: string; hint: string }[]>(() => [
  { kind: 'new', title: t('diff.new'), hint: t('diff.newHint') },
  { kind: 'modified', title: t('diff.modified'), hint: t('diff.modifiedHint') },
  { kind: 'missing', title: t('diff.gone'), hint: t('diff.goneHint') },
  { kind: 'unindexed', title: t('diff.pending'), hint: t('diff.pendingHint') }
])

const selected = ref<Set<string>>(new Set())

// 每次打开都从「未勾选」开始：不替用户预选任何一条
watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) selected.value = new Set()
  }
)

// P1-5：报告搜索过滤（此前条目只能肉眼看）
const searchQuery = ref('')
function matchesSearch(path: string): boolean {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return true
  return path.toLowerCase().includes(q)
}

function itemsOf(kind: DiffKind): DiffItem[] {
  return props.report?.items.filter((i) => i.kind === kind && matchesSearch(i.path)) ?? []
}

function toggle(path: string) {
  if (selected.value.has(path)) selected.value.delete(path)
  else selected.value.add(path)
}

function toggleGroup(kind: DiffKind) {
  const group = itemsOf(kind)
  const allSelected = group.length > 0 && group.every((i) => selected.value.has(i.path))
  if (allSelected) group.forEach((i) => selected.value.delete(i.path))
  else group.forEach((i) => selected.value.add(i.path))
}

/** 只有「新增 / 修改 / 待索引」可以被送去索引；「消失」只能标记失效或忽略 */
const indexableSelected = computed(() =>
  (props.report?.items ?? []).filter(
    (i) => selected.value.has(i.path) && i.kind !== 'missing'
  )
)
const missingSelected = computed(() =>
  (props.report?.items ?? []).filter(
    (i) => selected.value.has(i.path) && i.kind === 'missing'
  )
)

function emitIgnore() {
  const paths = [...selected.value]
  if (paths.length) emit('ignore', paths)
}

// P0-4：差异报告高危动作的二次确认
// 「加入索引」会覆盖旧帧并触发重编码（可能很久）；「标记失效」会改库（可逆）。
// 「忽略」纯前端、临时、无害，无需确认。
type DiffConfirm = { kind: 'index' | 'mark-dead'; paths: string[]; count: number }
const confirmOpen = ref(false)
const pendingConfirm = ref<DiffConfirm | null>(null)

function requestIndex() {
  const paths = indexableSelected.value.map((i) => i.path)
  if (!paths.length) return
  pendingConfirm.value = { kind: 'index', paths, count: paths.length }
  confirmOpen.value = true
}
function requestMarkDead() {
  const paths = missingSelected.value.map((i) => i.path)
  if (!paths.length) return
  pendingConfirm.value = { kind: 'mark-dead', paths, count: paths.length }
  confirmOpen.value = true
}
function onConfirmAction() {
  if (!pendingConfirm.value) return
  const { kind, paths } = pendingConfirm.value
  confirmOpen.value = false
  pendingConfirm.value = null
  if (kind === 'index') emit('index', paths)
  else emit('mark-dead', paths)
}

function fileName(path: string): string {
  const parts = path.split(/[\\/]/)
  return parts[parts.length - 1] || path
}
function dirName(path: string): string {
  const idx = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'))
  return idx > 0 ? path.slice(0, idx) : path
}
function formatSize(bytes: number | null): string {
  if (bytes === null) return '—'
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}
function formatTime(ms: number | null): string {
  if (ms === null) return '—'
  return new Date(ms).toLocaleString()
}
</script>

<template>
  <div v-if="props.open" class="diff-mask" @click.self="emit('close')">
    <div class="diff-panel">
      <header class="diff-head">
        <h2>{{ $t('diff.title') }}</h2>
        <p class="diff-hint">
          {{ $t('diff.readonlyHint') }}
        </p>
        <!-- P1-5：搜索过滤 -->
        <input
          v-model="searchQuery"
          type="text"
          class="report-search-input"
          :placeholder="$t('diff.searchPlaceholder')"
        />
      </header>

      <!-- 总览层 -->
      <div v-if="props.report" class="overview">
        <span class="ov-item">{{ $t('diff.new') }} <b>{{ props.report.new_count }}</b></span>
        <span class="ov-item">{{ $t('diff.modified') }} <b>{{ props.report.modified_count }}</b></span>
        <span class="ov-item">{{ $t('diff.gone') }} <b>{{ props.report.missing_count }}</b></span>
        <span class="ov-item">{{ $t('diff.pending') }} <b>{{ props.report.unindexed_count }}</b></span>
        <span class="ov-dim">
          {{ $t('diff.scannedFiles', { count: props.report.scanned_files }) }}
          <template v-if="props.report.baseline_batch_id">
            {{ $t('diff.baselineBatch', { id: props.report.baseline_batch_id }) }}
          </template>
          <template v-else>{{ $t('diff.noBaseline') }}</template>
        </span>
      </div>

      <p v-if="props.report?.mass_change_suspected" class="mass-warn">
        {{ $t('diff.mtimeWarning') }}
        {{ $t('diff.mtimeWarning2') }}
      </p>

      <div v-if="!props.report || props.report.items.length === 0" class="diff-empty">
        {{ $t('diff.noDiff') }}
      </div>

      <!-- 明细层 -->
      <section v-for="g in GROUPS" :key="g.kind" class="diff-group">
        <template v-if="itemsOf(g.kind).length">
          <div class="group-head">
            <label class="group-title">
              <input
                type="checkbox"
                :checked="itemsOf(g.kind).every((i) => selected.has(i.path))"
                @change="toggleGroup(g.kind)"
              />
              <span>{{ g.title }}（{{ itemsOf(g.kind).length }}）</span>
            </label>
            <span class="group-note">{{ g.hint }}</span>
          </div>

          <ul class="diff-list">
            <li
              v-for="item in itemsOf(g.kind)"
              :key="item.path"
              class="diff-row"
              :class="{ 'is-sel': selected.has(item.path) }"
            >
              <label class="row-main">
                <input
                  type="checkbox"
                  :checked="selected.has(item.path)"
                  @change="toggle(item.path)"
                />
                <span class="row-text">
                  <span class="row-name">{{ fileName(item.path) }}</span>
                  <span class="row-dir">{{ dirName(item.path) }}</span>
                </span>
              </label>
              <span class="row-meta">
                <span v-if="item.frame_count" class="badge">{{ $t('diff.frameCount', { count: item.frame_count }) }}</span>
                <span v-if="item.disk_size !== null" class="dim">
                  {{ $t('diff.nowSize', { size: formatSize(item.disk_size) }) }}
                </span>
                <span
                  v-if="item.observed_size !== null && item.observed_size !== item.disk_size"
                  class="dim"
                >
                  {{ $t('diff.origSize', { size: formatSize(item.observed_size) }) }}
                </span>
                <span v-if="item.disk_mtime !== null" class="dim">
                  {{ formatTime(item.disk_mtime) }}
                </span>
              </span>
            </li>
          </ul>
        </template>
      </section>

      <!-- 行动层 -->
      <footer class="diff-foot">
        <span v-if="props.message" class="foot-msg">{{ props.message }}</span>
        <span v-else-if="props.busy" class="foot-msg">{{ $t('common.processing') }}</span>
        <span v-else-if="selected.size" class="foot-msg">{{ $t('diff.selected', { count: selected.size }) }}</span>

        <div class="foot-actions">
          <button
            class="btn btn-primary"
            :disabled="props.busy || indexableSelected.length === 0"
            @click="requestIndex"
          >
            {{ $t('diff.index') }}
          </button>
          <button
            class="btn btn-secondary"
            :disabled="props.busy || missingSelected.length === 0"
            @click="requestMarkDead"
          >
            {{ $t('diff.markDead') }}
          </button>
          <button
            class="btn btn-ghost"
            :disabled="props.busy || selected.size === 0"
            @click="emitIgnore"
          >
            {{ $t('diff.ignore') }}
          </button>
          <button class="btn btn-secondary" @click="emit('close')">{{ $t('common.close') }}</button>
        </div>
      </footer>

      <!-- P0-4：差异报告高危动作二次确认 -->
      <ConfirmDialog
        :visible="confirmOpen"
        :title="pendingConfirm?.kind === 'index' ? $t('diff.indexConfirmTitle') : $t('diff.markDeadConfirmTitle')"
        :message="
          pendingConfirm?.kind === 'index'
            ? $t('diff.indexConfirmMessage', { count: pendingConfirm?.count ?? 0 })
            : $t('diff.markDeadConfirmMessage', { count: pendingConfirm?.count ?? 0 })
        "
        :confirm-text="pendingConfirm?.kind === 'index' ? $t('diff.index') : $t('diff.markDead')"
        :danger="pendingConfirm?.kind === 'index'"
        @confirm="onConfirmAction"
        @cancel="confirmOpen = false"
      />
    </div>
  </div>
</template>

<style scoped>
.diff-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.62);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
  padding: 24px;
}

.diff-panel {
  width: min(940px, 100%);
  max-height: 86vh;
  overflow-y: auto;
  background: #16161d;
  border: 1px solid #333344;
  border-radius: 10px;
  padding: 20px 22px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.diff-head h2 {
  margin: 0 0 6px;
  font-size: 18px;
  color: #fff;
}
.diff-hint {
  margin: 0;
  font-size: 13px;
  color: #9a9aae;
  line-height: 1.5;
}
.report-search-input {
  margin-top: 10px;
  width: 100%;
  box-sizing: border-box;
  padding: 8px 12px;
  background: #0c0c12;
  border: 1px solid #2a2a3a;
  border-radius: 6px;
  color: #f0f0f5;
  font-size: 12px;
  outline: none;
}
.report-search-input:focus {
  border-color: #6c8ee3;
}

.overview {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 14px;
  padding: 10px 14px;
  background: #121218;
  border: 1px solid #2a2a36;
  border-radius: 8px;
  font-size: 13px;
  color: #c8c8d8;
}
.ov-item b {
  color: #fff;
  font-size: 15px;
}
.ov-dim {
  margin-left: auto;
  font-size: 11px;
  color: #6f6f85;
}

.mass-warn {
  margin: 0;
  padding: 10px 14px;
  font-size: 12.5px;
  line-height: 1.55;
  color: #ffd9a0;
  background: #3a2c14;
  border: 1px solid #6b5320;
  border-radius: 8px;
}

.diff-empty {
  padding: 22px;
  text-align: center;
  font-size: 14px;
  color: #9a9aae;
  background: #121218;
  border: 1px dashed #333344;
  border-radius: 8px;
}

.diff-group {
  background: #121218;
  border: 1px solid #2a2a36;
  border-radius: 8px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.group-head {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 10px;
}
.group-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: #e8e8f0;
  cursor: pointer;
}
.group-note {
  font-size: 12px;
  color: #8a8a9c;
}

.diff-list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 220px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.diff-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 7px 8px;
  border-radius: 6px;
}
.diff-row:hover {
  background: #1c1c26;
}
.diff-row.is-sel {
  background: #1e2436;
}

.row-main {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  cursor: pointer;
  flex: 1 1 auto;
}
.row-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.row-name {
  font-size: 13px;
  color: #e8e8f0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.row-dir {
  font-size: 11px;
  color: #6f6f85;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 0 0 auto;
}
.badge {
  font-size: 11px;
  padding: 2px 7px;
  border-radius: 10px;
  background: #26263a;
  color: #b9b9d0;
}
.dim {
  font-size: 11px;
  color: #6f6f85;
}

.diff-foot {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
  padding-top: 10px;
  border-top: 1px solid #26262f;
}
.foot-msg {
  font-size: 12px;
  color: #9a9aae;
}
.foot-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-left: auto;
}

.btn {
  padding: 7px 14px;
  border-radius: 6px;
  border: 1px solid #333344;
  font-size: 13px;
  cursor: pointer;
  background: #1e1e28;
  color: #e8e8f0;
}
.btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.btn-primary {
  background: #2b4b7a;
  border-color: #3a5f95;
}
.btn-secondary {
  background: #23232e;
}
.btn-ghost {
  background: transparent;
  border-style: dashed;
  color: #9a9aae;
}
</style>
