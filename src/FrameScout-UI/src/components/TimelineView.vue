<script setup lang="ts">
/**
 * TimelineView — 时间轴 + 去年今天（P2-4 / 第三轮「时间轴可视化 / 去年今天」）。
 *
 * 数据来源：`file_events` 表的 `first_seen` 事件按日聚合（`list_timeline` 命令）。
 * 这是 Batch 2 就建好的账簿级数据，此前只被差异报告用，没被任何浏览界面用。
 *
 * 债单 A3/D12 修复：本组件原是孤儿（无父组件 import）。现改为弹窗形态，
 * 由 App.vue 通过 `visible` / `close` 控制，打开时加载数据。
 */
import { ref, watch, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '@/composables/useToast'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const { push } = useToast()
const { t } = useI18n()

interface TimelineDay {
  date: string
  count: number
  timestamp: number
}

const timeline = ref<TimelineDay[]>([])
const lastYearCount = ref(0)
const loading = ref(false)

// P2-9：时间轴下钻——点某天/「去年今天」看那天入库了哪些文件
interface DayFile {
  path: string
  timestamp: number
}
const dayFiles = ref<DayFile[]>([])
const dayFilesDate = ref('')
const dayFilesLoading = ref(false)

function localDateStr(d: Date): string {
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

async function loadDayFiles(date: string) {
  dayFilesDate.value = date
  dayFilesLoading.value = true
  try {
    dayFiles.value = await invoke<DayFile[]>('list_files_by_day', { date })
  } catch (err) {
    push(t('timeline.loadFailed', { err }), 'error')
  } finally {
    dayFilesLoading.value = false
  }
}

async function loadLastYearToday() {
  const d = new Date()
  d.setFullYear(d.getFullYear() - 1)
  await loadDayFiles(localDateStr(d))
}

function fileName(path: string): string {
  const parts = path.split(/[\\/]/)
  return parts[parts.length - 1] || path
}

const maxCount = computed(() => Math.max(1, ...timeline.value.map(d => d.count)))
const totalFiles = computed(() => timeline.value.reduce((sum, d) => sum + d.count, 0))

// 打开时加载（而非 onMounted，避免挂载即请求 + 重复打开不刷新）
watch(
  () => props.visible,
  async (visible) => {
    if (!visible) return
    loading.value = true
    try {
      const [tl, lyc] = await Promise.all([
        invoke<TimelineDay[]>('list_timeline', { days: 365 }),
        invoke<number>('last_year_today_count'),
      ])
      timeline.value = tl
      lastYearCount.value = lyc
    } catch (err) {
      push(t('timeline.loadFailed', { err }), 'error')
    } finally {
      loading.value = false
    }
  }
)

// 柱状图高度比例
function barHeight(count: number): number {
  return Math.max(2, (count / maxCount.value) * 100)
}

// 日期显示（MM-DD）
function shortDate(dateStr: string): string {
  return dateStr.slice(5) // "2026-10-07" → "10-07"
}
</script>

<template>
  <Teleport to="body">
    <div v-if="props.visible" class="timeline-overlay" @click.self="emit('close')">
      <div class="timeline-dialog">
        <header class="timeline-head">
          <h2 class="timeline-title">{{ $t('timeline.title') }}</h2>
          <button class="timeline-close" @click="emit('close')">✕</button>
        </header>

        <div class="timeline-body">
          <!-- 去年今天提示卡（P2-9：可点击下钻） -->
          <div v-if="lastYearCount > 0" class="last-year-card clickable" @click="loadLastYearToday">
            <span class="ly-icon">📅</span>
            <div class="ly-content">
              <p class="ly-title">{{ $t('timeline.lastYear') }}</p>
              <p class="ly-count">{{ $t('timeline.lastYearCount', { count: lastYearCount }) }}</p>
            </div>
          </div>

          <!-- 统计 -->
          <div class="timeline-stats">
            <span>{{ $t('timeline.summary', { total: totalFiles, days: timeline.length }) }}</span>
          </div>

          <!-- 柱状图 -->
          <div v-if="loading" class="timeline-loading">{{ $t('common.loading') }}</div>
          <div v-else-if="timeline.length === 0" class="timeline-empty">{{ $t('timeline.empty') }}</div>
          <div v-else class="timeline-chart">
            <div
              v-for="day in [...timeline].reverse()"
              :key="day.date"
              class="timeline-bar-wrapper clickable"
              :title="$t('timeline.dayCount', { date: day.date, count: day.count })"
              @click="loadDayFiles(day.date)"
            >
              <div
                class="timeline-bar"
                :style="{ height: barHeight(day.count) + '%' }"
              ></div>
              <span class="timeline-date">{{ shortDate(day.date) }}</span>
            </div>
          </div>

          <!-- P2-9：某天入库文件列表（下钻结果） -->
          <div v-if="dayFilesDate" class="day-files-block">
            <div class="day-files-head">
              <span class="day-files-title">{{ $t('timeline.dayFilesTitle', { date: dayFilesDate }) }}</span>
              <button class="day-files-close" @click="dayFilesDate = ''">✕</button>
            </div>
            <div v-if="dayFilesLoading" class="day-files-loading">{{ $t('common.loading') }}</div>
            <ul v-else-if="dayFiles.length" class="day-files-list">
              <li v-for="f in dayFiles" :key="f.path" class="day-file-row">
                <span class="day-file-name" :title="f.path">{{ fileName(f.path) }}</span>
                <span class="day-file-dir">{{ f.path }}</span>
              </li>
            </ul>
            <p v-else class="day-files-empty">{{ $t('timeline.dayFilesEmpty') }}</p>
          </div>
        </div>

        <footer class="timeline-foot">
          <button class="btn-done" @click="emit('close')">{{ $t('common.done') }}</button>
        </footer>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.timeline-overlay {
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

.timeline-dialog {
  width: 720px;
  max-width: 92vw;
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  background: linear-gradient(180deg, rgba(28, 28, 46, 0.98), rgba(20, 20, 30, 0.98));
  border: 1px solid rgba(108, 142, 227, 0.3);
  border-radius: 16px;
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
  overflow: hidden;
}

.timeline-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.timeline-title {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: #f0f0f5;
}

.timeline-close {
  background: none;
  border: none;
  color: #8888a0;
  font-size: 18px;
  cursor: pointer;
  padding: 4px 10px;
  border-radius: 6px;
  transition: background 0.15s, color 0.15s;
}

.timeline-close:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #f0f0f5;
}

.timeline-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.last-year-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px 20px;
  background: linear-gradient(135deg, rgba(108, 142, 227, 0.12), rgba(139, 127, 232, 0.08));
  border: 1px solid rgba(108, 142, 227, 0.3);
  border-radius: 12px;
}

.clickable {
  cursor: pointer;
}

/* P2-9：某天入库文件列表（下钻结果） */
.day-files-block {
  background: #121218;
  border: 1px solid #2a2a36;
  border-radius: 8px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.day-files-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.day-files-title {
  font-size: 13px;
  font-weight: 600;
  color: #e8e8f0;
}
.day-files-close {
  background: none;
  border: none;
  color: #8888a0;
  cursor: pointer;
  font-size: 14px;
}
.day-files-loading {
  font-size: 12px;
  color: #666677;
}
.day-files-list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 200px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.day-file-row {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 6px 8px;
  background: rgba(10, 10, 12, 0.4);
  border-radius: 6px;
}
.day-file-name {
  font-size: 12px;
  color: #c8c8d8;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.day-file-dir {
  font-size: 11px;
  color: #6f6f85;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.day-files-empty {
  margin: 0;
  font-size: 12px;
  color: #666677;
}

.ly-icon {
  font-size: 32px;
}

.ly-content p {
  margin: 0;
}

.ly-title {
  font-size: 14px;
  color: #8888a0;
}

.ly-count {
  font-size: 18px;
  font-weight: 600;
  color: #f0f0f5;
}

.timeline-stats {
  font-size: 13px;
  color: #8888a0;
}

.timeline-loading,
.timeline-empty {
  text-align: center;
  padding: 40px;
  color: #666677;
  font-size: 14px;
}

.timeline-chart {
  display: flex;
  align-items: flex-end;
  gap: 2px;
  height: 120px;
  overflow-x: auto;
  padding-bottom: 20px;
}

.timeline-bar-wrapper {
  display: flex;
  flex-direction: column;
  align-items: center;
  flex-shrink: 0;
  min-width: 24px;
  height: 100%;
  justify-content: flex-end;
}

.timeline-bar {
  width: 8px;
  background: linear-gradient(180deg, #6c8ee3, #4a6fcf);
  border-radius: 2px 2px 0 0;
  min-height: 2px;
  transition: opacity 0.15s, transform 0.15s;
}

.timeline-bar-wrapper:hover .timeline-bar {
  opacity: 0.8;
  transform: scaleY(1.05);
}

.timeline-date {
  font-size: 9px;
  color: #555566;
  margin-top: 4px;
  white-space: nowrap;
}

.timeline-foot {
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
</style>
