<script setup lang="ts">
/**
 * 扫描报告 / 活动记录。
 *
 * 数据全部来自 `batches` 表（骨架 + summary_json），不额外记账。
 * 合法完成 = finished_at 非空；崩溃残留会显示成 interrupted，
 * 让用户能看见「那次没跑完」，而不是假装什么都没发生。
 */
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { BatchReport } from '@/types/search'
import { useToast } from '@/composables/useToast'

const props = defineProps<{
  open: boolean
  reports: BatchReport[]
  message: string
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const expanded = ref<Set<number>>(new Set())

function toggle(id: number) {
  if (expanded.value.has(id)) expanded.value.delete(id)
  else expanded.value.add(id)
}

const { t } = useI18n()
const { push } = useToast()

// P1-5：复制失败文件列表（此前只能肉眼看，无法复制去重试）
async function copyFailedPaths(paths: string[]) {
  try {
    await navigator.clipboard.writeText(paths.join('\n'))
    push(t('cards.copied'), 'success')
  } catch {
    push(t('cards.copyFailed'), 'error')
  }
}

const TYPE_META: Record<string, { icon: string; labelKey: string }> = {
  scan: { icon: '🚀', labelKey: 'report.scan' },
  clean_ghosts: { icon: '🧹', labelKey: 'report.ghostCleanup' },
  reindex: { icon: '♻️', labelKey: 'report.reindex' },
  legacy_import: { icon: '📦', labelKey: 'report.existingImport' }
}

function meta(type: string) {
  const m = TYPE_META[type]
  return m ? { icon: m.icon, label: t(m.labelKey) } : { icon: '📄', label: type }
}

interface ParsedSummary {
  pairs: [string, string][]
  failedPaths: string[]
  topDirs: { dir: string; count: number }[]
}

function parseSummary(raw: string | null): ParsedSummary | null {
  if (!raw) return null
  let obj: Record<string, unknown>
  try {
    obj = JSON.parse(raw) as Record<string, unknown>
  } catch {
    return null
  }

  const LABELS: Record<string, string> = {
    requested: 'report.requests',
    frames_inserted: 'report.framesWritten',
    batches: 'report.batches',
    first_seen: 'report.firstIndexed',
    observed_change: 'report.changesObserved',
    failed_count: 'report.failed',
    took_ms: 'report.totalTime',
    avg_ms_per_file: 'report.avgPerFile',
    succeeded: 'report.succeeded',
    replaced_frames: 'report.replacedFrames',
    new_frames: 'report.newFrames',
    affected: 'report.affectedEntries',
    affected_frames: 'report.affectedFrames',
    seeded_paths: 'report.pathsFilled'
  }
  const UNITS: Record<string, string> = { took_ms: 'ms', avg_ms_per_file: 'ms' }

  const pairs: [string, string][] = []
  for (const [key, value] of Object.entries(obj)) {
    if (key === 'failed_paths' || key === 'skipped' || key === 'top_dirs') continue
    const label = LABELS[key] ? t(LABELS[key]) : key
    const unit = UNITS[key] ?? ''
    pairs.push([label, `${value}${unit}`])
  }

  const failedPaths = Array.isArray(obj.failed_paths)
    ? (obj.failed_paths as unknown[]).map(String)
    : []
  const topDirs = Array.isArray(obj.top_dirs)
    ? (obj.top_dirs as { dir?: string; count?: number }[]).map((d) => ({
        dir: String(d?.dir ?? ''),
        count: Number(d?.count ?? 0)
      }))
    : []

  return { pairs, failedPaths, topDirs }
}

function statusLabel(r: BatchReport): string {
  if (r.status === 'interrupted') return t('report.incomplete')
  if (!r.finished_at) return t('report.inProgress')
  return t('report.completed')
}
</script>

<template>
  <div v-if="props.open" class="report-mask" @click.self="emit('close')">
    <div class="report-panel">
      <header class="report-head">
        <h2>{{ $t('report.title') }}</h2>
        <p class="report-hint">
          {{ $t('report.hint') }}
        </p>
      </header>

      <div v-if="props.reports.length === 0" class="report-empty">
        {{ $t('report.empty') }}
      </div>

      <ul class="report-list">
        <li v-for="r in props.reports" :key="r.id" class="report-row">
          <button class="row-head" @click="toggle(r.id)">
            <span class="row-icon">{{ meta(r.batch_type).icon }}</span>
            <span class="row-title">
              <span class="row-kind">{{ meta(r.batch_type).label }}</span>
              <span class="row-id">{{ r.batch_id }}</span>
            </span>
            <span class="row-status" :class="{ 'is-bad': r.status === 'interrupted' }">
              {{ statusLabel(r) }}
            </span>
            <span class="row-time">{{ r.finished_at ?? r.started_at ?? '—' }}</span>
            <span class="row-caret">{{ expanded.has(r.id) ? '▾' : '▸' }}</span>
          </button>

          <div v-if="expanded.has(r.id)" class="row-body">
            <template v-if="parseSummary(r.summary_json)">
              <div class="kv-grid">
                <template v-for="[k, v] in parseSummary(r.summary_json)!.pairs" :key="k">
                  <span class="kv-k">{{ k }}</span>
                  <span class="kv-v">{{ v }}</span>
                </template>
              </div>

              <div v-if="parseSummary(r.summary_json)!.topDirs.length" class="sub-block">
                <div class="sub-title">{{ $t('report.dirDistribution') }}</div>
                <div
                  v-for="d in parseSummary(r.summary_json)!.topDirs"
                  :key="d.dir"
                  class="dir-row"
                >
                  <span class="dir-path">{{ d.dir }}</span>
                  <span class="dir-count">{{ d.count }}</span>
                </div>
              </div>

              <div v-if="parseSummary(r.summary_json)!.failedPaths.length" class="sub-block">
                <div class="sub-title-row">
                  <div class="sub-title">{{ $t('report.failedFiles', { count: parseSummary(r.summary_json)!.failedPaths.length }) }}</div>
                  <button
                    class="btn-text"
                    @click="copyFailedPaths(parseSummary(r.summary_json)!.failedPaths)"
                  >{{ $t('cards.copy') }}</button>
                </div>
                <div
                  v-for="p in parseSummary(r.summary_json)!.failedPaths"
                  :key="p"
                  class="fail-path"
                >
                  {{ p }}
                </div>
              </div>
            </template>
            <div v-else class="no-summary">{{ $t('report.noSummary') }}</div>
          </div>
        </li>
      </ul>

      <footer class="report-foot">
        <span v-if="props.message" class="foot-msg">{{ props.message }}</span>
        <button class="btn btn-secondary" @click="emit('close')">{{ $t('common.close') }}</button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.report-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.62);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
  padding: 24px;
}

.report-panel {
  width: min(820px, 100%);
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

.report-head h2 {
  margin: 0 0 6px;
  font-size: 18px;
  color: #fff;
}
.report-hint {
  margin: 0;
  font-size: 13px;
  color: #9a9aae;
}

.report-empty {
  padding: 22px;
  text-align: center;
  font-size: 14px;
  color: #9a9aae;
  background: #121218;
  border: 1px dashed #333344;
  border-radius: 8px;
}

.report-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.report-row {
  background: #121218;
  border: 1px solid #2a2a36;
  border-radius: 8px;
  overflow: hidden;
}

.row-head {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: transparent;
  border: none;
  cursor: pointer;
  text-align: left;
  color: inherit;
}
.row-head:hover {
  background: #1c1c26;
}

.row-icon {
  font-size: 15px;
}
.row-title {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1 1 auto;
}
.row-kind {
  font-size: 13px;
  color: #e8e8f0;
}
.row-id {
  font-size: 11px;
  color: #6f6f85;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.row-status {
  font-size: 12px;
  color: #7fd6a0;
  flex: 0 0 auto;
}
.row-status.is-bad {
  color: #e8a86a;
}
.row-time {
  font-size: 11px;
  color: #6f6f85;
  flex: 0 0 auto;
}
.row-caret {
  font-size: 11px;
  color: #6f6f85;
}

.row-body {
  padding: 12px;
  border-top: 1px solid #26262f;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.kv-grid {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 4px 14px;
  font-size: 12.5px;
}
.kv-k {
  color: #8a8a9c;
}
.kv-v {
  color: #e8e8f0;
}

.sub-block {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.sub-title {
  font-size: 12px;
  color: #8a8a9c;
}
.sub-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}
.dir-row {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  font-size: 12px;
}
.dir-path {
  color: #c8c8d8;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.dir-count {
  color: #8a8a9c;
  flex: 0 0 auto;
}
.fail-path {
  font-size: 11.5px;
  color: #e8a86a;
  word-break: break-all;
}

.no-summary {
  font-size: 12px;
  color: #6f6f85;
}

.report-foot {
  display: flex;
  align-items: center;
  gap: 10px;
}
.foot-msg {
  font-size: 12px;
  color: #9a9aae;
  margin-right: auto;
}

.btn {
  padding: 7px 14px;
  border-radius: 6px;
  border: 1px solid #333344;
  font-size: 13px;
  cursor: pointer;
  background: #23232e;
  color: #e8e8f0;
}
</style>
