<script setup lang="ts">
/**
 * ConditionRow — 单条搜索条件行（第七轮逻辑搜索重构；第八轮补齐类型与快捷项）。
 * 一行 = 序号 + 类型下拉 + 值控件（随类型变化）+ 作用域 + 阈值 + ✕。
 * 每一行都可改、可删，用户加的就能删。
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { open } from '@tauri-apps/plugin-dialog'
import type { ConditionRow, ConditionType, ConditionTarget, BatchReport } from '@/types/search'

const props = defineProps<{
  condition: ConditionRow
  index: number
  batches: BatchReport[]
}>()

const emit = defineEmits<{
  (e: 'update', patch: Partial<ConditionRow>): void
  (e: 'remove'): void
}>()

const { t } = useI18n()

const typeOptions: { value: ConditionType; label: string }[] = [
  { value: 'text', label: t('cond.typeText') },
  { value: 'visual', label: t('cond.typeVisual') },
  { value: 'exclude', label: t('cond.typeExclude') },
  { value: 'time', label: t('cond.typeTime') },
  { value: 'path', label: t('cond.typePath') },
  { value: 'batch', label: t('cond.typeBatch') },
  { value: 'file_type', label: t('cond.typeFileType') },
  { value: 'seed', label: t('cond.typeSeed') },
  { value: 'text_semantic', label: t('cond.typeTextSemantic') },
  { value: 'text_entry', label: t('cond.typeTextEntry') }
]

const targetOptions: { value: ConditionTarget; label: string }[] = [
  { value: 'filename', label: t('search.filename') },
  { value: 'note', label: t('search.note') },
  { value: 'ocr', label: t('search.ocr') }
]

const fileTypeOptions: { value: string; label: string }[] = [
  { value: 'image', label: t('cond.typeImages') },
  { value: 'video', label: t('cond.typeVideos') }
]

const isTextLike = computed(() => props.condition.type === 'text' || props.condition.type === 'exclude')
const isValueInput = computed(() =>
  ['text', 'exclude', 'visual', 'text_semantic', 'text_entry'].includes(props.condition.type)
)
const isVisual = computed(() => props.condition.type === 'visual' || props.condition.type === 'seed')
const isTime = computed(() => props.condition.type === 'time')
const isPath = computed(() => props.condition.type === 'path')
const isBatch = computed(() => props.condition.type === 'batch')
const isFileType = computed(() => props.condition.type === 'file_type')
const isSeed = computed(() => props.condition.type === 'seed')

const CIRCLED = ['①', '②', '③', '④', '⑤', '⑥', '⑦', '⑧', '⑨', '⑩']
const indexLabel = computed(() => {
  const i = props.index
  return i >= 1 && i <= CIRCLED.length ? CIRCLED[i - 1] : String(i)
})

function onTypeChange(e: Event) {
  const type = (e.target as HTMLSelectElement).value as ConditionType
  const patch: Partial<ConditionRow> = { type }
  if (type === 'text' || type === 'exclude') {
    patch.targets = ['filename', 'note', 'ocr']
  } else if (type === 'visual') {
    patch.minScore = null
    patch.targets = []
  } else if (type === 'seed') {
    patch.minScore = null
    patch.seedPath = ''
  } else if (type === 'time') {
    patch.timeFrom = null
    patch.timeTo = null
  } else if (type === 'path') {
    patch.pathPrefix = ''
  } else if (type === 'batch') {
    patch.batchId = ''
  } else if (type === 'file_type') {
    patch.fileTypes = []
  }
  emit('update', patch)
}

function onValueInput(e: Event) {
  emit('update', { value: (e.target as HTMLInputElement).value })
}

function toggleTarget(target: ConditionTarget, e: Event) {
  const checked = (e.target as HTMLInputElement).checked
  const cur = props.condition.targets ?? []
  const next = checked ? [...cur, target] : cur.filter((x) => x !== target)
  emit('update', { targets: next })
}

function toggleFileType(kind: string, e: Event) {
  const checked = (e.target as HTMLInputElement).checked
  const cur = props.condition.fileTypes ?? []
  const next = checked ? [...cur, kind] : cur.filter((x) => x !== kind)
  emit('update', { fileTypes: next })
}

function onMinScoreInput(e: Event) {
  const v = Number((e.target as HTMLInputElement).value)
  emit('update', { minScore: Number.isNaN(v) ? null : v })
}

function onPathInput(e: Event) {
  emit('update', { pathPrefix: (e.target as HTMLInputElement).value })
}

function onBatchChange(e: Event) {
  emit('update', { batchId: (e.target as HTMLSelectElement).value })
}

/** 第八轮：基准图条件（seed）此前只有后端实现、UI 不可达，现补上图片选择。 */
async function pickSeedImage() {
  const selected = await open({
    multiple: false,
    filters: [{ name: 'Images', extensions: ['png', 'jpeg', 'jpg', 'webp'] }]
  })
  if (selected) emit('update', { seedPath: selected as string })
}

/** M1''''：批次下拉用人类可读标签（日期 + 文件数），而非机器串 `scan_20261002_103747`。 */
function formatBatchLabel(b: BatchReport): string {
  const date = (b.started_at || '').slice(0, 10)
  const n = parseBatchCount(b.summary_json)
  return n != null ? `${date} · ${n} ${t('cond.files')}` : date
}
function parseBatchCount(summary: string | null): number | null {
  if (!summary) return null
  try {
    const o = JSON.parse(summary) as Record<string, unknown>
    const v = o.requested ?? o.first_seen ?? o.frames_inserted
    return typeof v === 'number' ? v : null
  } catch {
    return null
  }
}

function formatDateStr(unix: number | null | undefined): string {
  if (!unix) return ''
  const d = new Date(unix * 1000)
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

const fromDateStr = computed(() => formatDateStr(props.condition.timeFrom))
const toDateStr = computed(() => formatDateStr(props.condition.timeTo))

function dayStart(d: Date): number {
  return Math.floor(new Date(d.getFullYear(), d.getMonth(), d.getDate(), 0, 0, 0, 0).getTime() / 1000)
}

function onFromChange(e: Event) {
  const v = (e.target as HTMLInputElement).value
  const from = v ? new Date(v + 'T00:00:00').getTime() / 1000 : null
  // 「到」早于「从」时，把「到」一起推到「从」当天末尾 —— 避免空结果无解释。
  const patch: Partial<ConditionRow> = { timeFrom: from }
  if (from != null && props.condition.timeTo != null && props.condition.timeTo < from) {
    patch.timeTo = from + 86399
  }
  emit('update', patch)
}
function onToChange(e: Event) {
  const v = (e.target as HTMLInputElement).value
  const to = v ? new Date(v + 'T23:59:59').getTime() / 1000 : null
  const patch: Partial<ConditionRow> = { timeTo: to }
  if (to != null && props.condition.timeFrom != null && props.condition.timeFrom > to) {
    patch.timeFrom = to - 86399
  }
  emit('update', patch)
}

/** 第八轮：日期快捷项（今天 / 7 天 / 30 天 / 本月 / 本年）—— 用户不必手搓日期控件。 */
type Shortcut = 'today' | 'd7' | 'd30' | 'month' | 'year'
function applyShortcut(kind: Shortcut) {
  const now = new Date()
  const todayStart = dayStart(now)
  const todayEnd = todayStart + 86399
  let from: number
  let to: number | null = null
  if (kind === 'today') {
    from = todayStart
    to = todayEnd
  } else if (kind === 'd7') {
    // 第八轮核查修复：原实现 `from = dayStart(now)`、`to = from + 7*86400`，
    // 算的是「今天起未来 7 天」，而 index_time 不可能大于 now ⇒ 实际只命中今天。
    // 现改为「过去 6 天 + 今天」（共 7 天）。
    from = todayStart - 6 * 86400
    to = todayEnd
  } else if (kind === 'd30') {
    from = todayStart - 29 * 86400
    to = todayEnd
  } else if (kind === 'month') {
    from = dayStart(new Date(now.getFullYear(), now.getMonth(), 1))
    to = dayStart(new Date(now.getFullYear(), now.getMonth() + 1, 1)) - 1
  } else {
    from = dayStart(new Date(now.getFullYear(), 0, 1))
    to = dayStart(new Date(now.getFullYear() + 1, 0, 1)) - 1
  }
  emit('update', { timeFrom: from, timeTo: to })
}
</script>

<template>
  <div class="cond-row" :class="{ 'is-exclude': props.condition.type === 'exclude' }">
    <span class="cond-index">{{ indexLabel }}</span>

    <select class="cond-select" :value="props.condition.type" @change="onTypeChange">
      <option v-for="opt in typeOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
    </select>

    <input
      v-if="isValueInput"
      class="cond-input"
      type="text"
      :value="props.condition.value || ''"
      :placeholder="t('cond.valuePlaceholder')"
      @input="onValueInput"
    />

    <template v-if="isTextLike">
      <span class="cond-scope-label">{{ t('cond.in') }}</span>
      <label v-for="tg in targetOptions" :key="tg.value" class="cond-target">
        <input
          type="checkbox"
          :checked="(props.condition.targets ?? []).includes(tg.value)"
          @change="toggleTarget(tg.value, $event)"
        />
        {{ tg.label }}
      </label>
    </template>

    <!-- 第八轮：画面条件可附加文字通道（视觉为主 + 字面补充），此前该配置 UI 不可达。 -->
    <template v-if="props.condition.type === 'visual'">
      <span class="cond-scope-label">{{ t('cond.alsoText') }}</span>
      <label v-for="tg in targetOptions" :key="tg.value" class="cond-target">
        <input
          type="checkbox"
          :checked="(props.condition.targets ?? []).includes(tg.value)"
          @change="toggleTarget(tg.value, $event)"
        />
        {{ tg.label }}
      </label>
    </template>

    <template v-if="isSeed">
      <button class="cond-pick" type="button" @click="pickSeedImage">{{ t('cond.pickImage') }}</button>
      <span v-if="props.condition.seedPath" class="cond-seed-path" :title="props.condition.seedPath">
        {{ props.condition.seedPath }}
      </span>
    </template>

    <template v-if="isFileType">
      <label v-for="ft in fileTypeOptions" :key="ft.value" class="cond-target">
        <input
          type="checkbox"
          :checked="(props.condition.fileTypes ?? []).includes(ft.value)"
          @change="toggleFileType(ft.value, $event)"
        />
        {{ ft.label }}
      </label>
    </template>

    <template v-if="isVisual">
      <span class="cond-scope-label">{{ t('cond.minScore') }}</span>
      <input
        class="cond-num"
        type="number"
        min="0"
        max="100"
        :value="props.condition.minScore ?? ''"
        :placeholder="t('cond.noLimit')"
        @input="onMinScoreInput"
      />
    </template>

    <template v-if="isTime">
      <div class="cond-shortcuts">
        <button type="button" class="cond-chip" @click="applyShortcut('today')">{{ t('cond.today') }}</button>
        <button type="button" class="cond-chip" @click="applyShortcut('d7')">{{ t('cond.d7') }}</button>
        <button type="button" class="cond-chip" @click="applyShortcut('d30')">{{ t('cond.d30') }}</button>
        <button type="button" class="cond-chip" @click="applyShortcut('month')">{{ t('cond.thisMonth') }}</button>
        <button type="button" class="cond-chip" @click="applyShortcut('year')">{{ t('cond.thisYear') }}</button>
      </div>
      <input
        type="date"
        class="cond-date"
        :value="fromDateStr"
        :max="toDateStr || undefined"
        :title="t('search.fromDate')"
        @change="onFromChange"
      />
      <span class="cond-sep">–</span>
      <input
        type="date"
        class="cond-date"
        :value="toDateStr"
        :min="fromDateStr || undefined"
        :title="t('search.toDate')"
        @change="onToChange"
      />
    </template>

    <template v-if="isPath">
      <input
        class="cond-input"
        type="text"
        :value="props.condition.pathPrefix || ''"
        :placeholder="t('cond.pathPlaceholder')"
        @input="onPathInput"
      />
    </template>

    <template v-if="isBatch">
      <select class="cond-select" :value="props.condition.batchId || ''" @change="onBatchChange">
        <option value="">{{ t('search.allSources') }}</option>
        <option v-for="b in batches" :key="b.batch_id" :value="b.batch_id">{{ formatBatchLabel(b) }}</option>
      </select>
    </template>

    <button class="cond-remove" :title="t('cond.remove')" @click="emit('remove')">✕</button>
  </div>
</template>

<style scoped>
.cond-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  padding: 8px 10px;
  background: rgba(18, 18, 28, 0.6);
  border: 1px solid rgba(108, 142, 227, 0.15);
  border-radius: 8px;
  transition: border-color 0.15s;
}
.cond-row:hover {
  border-color: rgba(108, 142, 227, 0.35);
}
.cond-row.is-exclude {
  border-color: rgba(248, 113, 113, 0.25);
  background: rgba(40, 16, 20, 0.5);
}

.cond-index {
  font-size: 12px;
  color: #6c8ee3;
  font-weight: 600;
  min-width: 20px;
  text-align: center;
}

.cond-select,
.cond-input,
.cond-num,
.cond-date {
  padding: 6px 10px;
  background: rgba(10, 10, 14, 0.6);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  color: #f0f0f5;
  font-size: 13px;
  outline: none;
  font-family: 'Noto Sans', system-ui, sans-serif;
}
.cond-select:focus,
.cond-input:focus,
.cond-num:focus,
.cond-date:focus {
  border-color: rgba(108, 142, 227, 0.5);
}

.cond-input {
  flex: 1 1 160px;
  min-width: 120px;
}
.cond-num {
  width: 70px;
}
.cond-date {
  width: 140px;
  color-scheme: dark;
}

.cond-pick {
  padding: 5px 10px;
  background: rgba(108, 142, 227, 0.16);
  border: 1px solid rgba(108, 142, 227, 0.4);
  border-radius: 6px;
  color: #cdd8f7;
  font-size: 12px;
  cursor: pointer;
}
.cond-pick:hover {
  background: rgba(108, 142, 227, 0.28);
}
.cond-seed-path {
  font-size: 11px;
  color: #8a8aa0;
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cond-shortcuts {
  display: inline-flex;
  gap: 4px;
  flex-wrap: wrap;
}
.cond-chip {
  padding: 3px 8px;
  font-size: 11px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(255, 255, 255, 0.04);
  color: #b0b0c0;
  cursor: pointer;
}
.cond-chip:hover {
  color: #f0f0f5;
  border-color: rgba(108, 142, 227, 0.5);
}

.cond-scope-label {
  font-size: 12px;
  color: #8888a0;
}

.cond-target {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: #b0b0c0;
  cursor: pointer;
  white-space: nowrap;
}

.cond-sep {
  color: #666;
  font-size: 12px;
}

.cond-remove {
  background: none;
  border: none;
  color: #666677;
  font-size: 13px;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 50%;
  margin-left: auto;
  transition: color 0.15s, background 0.15s;
}
.cond-remove:hover {
  color: #f87171;
  background: rgba(248, 113, 113, 0.15);
}
</style>