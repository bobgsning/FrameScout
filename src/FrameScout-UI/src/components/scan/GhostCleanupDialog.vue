<script setup lang="ts">
/**
 * 幽灵清理对话框 —— 先预览、后行动。
 *
 * 设计原则：系统可以发现问题，但绝不替用户决定。
 *  · 打开时只做一次只读预览，不写任何状态；
 *  · 每个分组都提供可逆动作（标记失效 ←→ 回库）与终态动作（真删除）；
 *  · 真删除前明确展示「将失去多少向量行」。
 */
import { computed, ref, watch } from 'vue'
import type { GhostItem } from '@/types/search'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

const props = defineProps<{
  open: boolean
  items: GhostItem[]
  busy: boolean
  /** 上一次动作的结果回显（成功条数 / 跳过原因 / 失败原因） */
  message: string
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'apply', action: 'mark_dead' | 'restore' | 'purge', paths: string[]): void
}>()

/** 磁盘上已找不到、但仍在检索中的记录（排除离线盘——离线盘的 exists_on_disk 也为 false，但那是「盘没插」不是「文件没了」） */
const missing = computed(() => props.items.filter((i) => !i.is_dead && !i.exists_on_disk && !i.disk_offline && matchesSearch(i.path)))
/** 已标记失效、仍占着库的记录 */
const dead = computed(() => props.items.filter((i) => i.is_dead && matchesSearch(i.path)))
/** 已标记失效、且磁盘上又出现了（可以回库了） */
const backOnDisk = computed(() => dead.value.filter((i) => i.exists_on_disk))
/** 所在盘当前不可达（债单 A9/D6：NAS 未挂载 / 移动硬盘未插）。整组灰显、不可勾选、不可 purge，避免误删只是暂时离线的整盘 */
const offline = computed(() => props.items.filter((i) => i.disk_offline && matchesSearch(i.path)))

// P1-5：报告搜索过滤（此前几百条幽灵只能肉眼看）
const searchQuery = ref('')
function matchesSearch(path: string): boolean {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return true
  return path.toLowerCase().includes(q)
}

const selectedMissing = ref<Set<string>>(new Set())
const selectedDead = ref<Set<string>>(new Set())

// 每次重新打开都从「未勾选」开始：默认不做任何事，符合「不替用户决定」
watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      selectedMissing.value = new Set()
      selectedDead.value = new Set()
    }
  }
)

function toggle(set: Set<string>, path: string) {
  if (set.has(path)) set.delete(path)
  else set.add(path)
}

function toggleAll(group: GhostItem[], set: Set<string>) {
  if (set.size === group.length) set.clear()
  else group.forEach((i) => set.add(i.path))
}

function apply(action: 'mark_dead' | 'restore' | 'purge', set: Set<string>) {
  if (set.size === 0 || props.busy) return
  // P0-4：purge 是终态、不可逆（后端级联删 5 张表），必须二次确认；
  // mark_dead / restore 可逆，无需确认。
  if (action === 'purge') {
    const paths = [...set]
    const rows = props.items.filter((i) => paths.includes(i.path)).reduce((s, i) => s + i.frame_count, 0)
    pendingPurge.value = { paths, rows }
    confirmOpen.value = true
    return
  }
  emit('apply', action, [...set])
}

// P0-4：purge 二次确认
const confirmOpen = ref(false)
const pendingPurge = ref<{ paths: string[]; rows: number } | null>(null)
function onConfirmPurge() {
  if (pendingPurge.value) emit('apply', 'purge', pendingPurge.value.paths)
  confirmOpen.value = false
  pendingPurge.value = null
}

function formatSize(bytes: number | null): string {
  if (bytes === null) return '—'
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

function fileName(path: string): string {
  const parts = path.split(/[\\/]/)
  return parts[parts.length - 1] || path
}

function dirName(path: string): string {
  const idx = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'))
  return idx > 0 ? path.slice(0, idx) : path
}
</script>

<template>
  <div v-if="props.open" class="ghost-mask" @click.self="emit('close')">
    <div class="ghost-panel">
      <header class="ghost-head">
        <h2>{{ $t('ghost.title') }}</h2>
        <p class="ghost-hint">
          {{ $t('ghost.readonlyHint') }}
        </p>
        <!-- P1-5：搜索过滤 -->
        <input
          v-model="searchQuery"
          type="text"
          class="report-search-input"
          :placeholder="$t('ghost.searchPlaceholder')"
        />
      </header>

      <div v-if="props.items.length === 0" class="ghost-empty">
        {{ $t('ghost.nothingToDo') }}
      </div>

      <!-- 组 0：所在盘当前不可达（离线盘）——灰显、不可操作（债单 A9/D6） -->
      <section v-if="offline.length" class="ghost-group ghost-group-offline">
        <div class="group-head">
          <span class="group-title offline-title">{{ $t('ghost.offlineCount', { count: offline.length }) }}</span>
          <span class="group-note">{{ $t('ghost.offlineHint') }}</span>
        </div>
        <ul class="ghost-list">
          <li v-for="item in offline" :key="item.path" class="ghost-row ghost-row-disabled">
            <span class="row-text">
              <span class="row-name">{{ fileName(item.path) }}</span>
              <span class="row-dir">{{ dirName(item.path) }}</span>
            </span>
            <span class="row-meta">
              <span class="badge">{{ $t('ghost.frameCount', { count: item.frame_count }) }}</span>
              <span class="dim">{{ formatSize(item.observed_size) }}</span>
            </span>
          </li>
        </ul>
        <div class="offline-note">{{ $t('ghost.offlineExcluded') }}</div>
      </section>

      <!-- 组 1：磁盘已不在 -->
      <section v-if="missing.length" class="ghost-group">
        <div class="group-head">
          <label class="group-title">
            <input
              type="checkbox"
              :checked="selectedMissing.size === missing.length && missing.length > 0"
              @change="toggleAll(missing, selectedMissing)"
            />
            <span>{{ $t('ghost.missingCount', { count: missing.length }) }}</span>
          </label>
          <span class="group-note">{{ $t('ghost.missingHint') }}</span>
        </div>

        <ul class="ghost-list">
          <li v-for="item in missing" :key="item.path" class="ghost-row">
            <label class="row-main">
              <input
                type="checkbox"
                :checked="selectedMissing.has(item.path)"
                @change="toggle(selectedMissing, item.path)"
              />
              <span class="row-text">
                <span class="row-name">{{ fileName(item.path) }}</span>
                <span class="row-dir">{{ dirName(item.path) }}</span>
              </span>
            </label>
            <span class="row-meta">
              <span class="badge">{{ $t('ghost.frameCount', { count: item.frame_count }) }}</span>
              <span class="dim">{{ formatSize(item.observed_size) }}</span>
              <span class="dim">{{ $t('ghost.lastSeen', { value: item.last_seen_at ?? '—' }) }}</span>
            </span>
          </li>
        </ul>

        <div class="group-actions">
          <button
            class="btn btn-secondary"
            :disabled="props.busy || selectedMissing.size === 0"
            @click="apply('mark_dead', selectedMissing)"
          >
            {{ $t('ghost.markDead') }}
          </button>
          <button
            class="btn btn-danger"
            :disabled="props.busy || selectedMissing.size === 0"
            @click="apply('purge', selectedMissing)"
          >
            {{ $t('ghost.purge') }}
          </button>
          <span v-if="selectedMissing.size" class="sel-count">
            {{ $t('ghost.selectedMissing', { count: selectedMissing.size, rows: missing.filter((i) => selectedMissing.has(i.path)).reduce((s, i) => s + i.frame_count, 0) }) }}
          </span>
        </div>
      </section>

      <!-- 组 2：已标记失效（占库） -->
      <section v-if="dead.length" class="ghost-group">
        <div class="group-head">
          <label class="group-title">
            <input
              type="checkbox"
              :checked="selectedDead.size === dead.length && dead.length > 0"
              @change="toggleAll(dead, selectedDead)"
            />
            <span>{{ $t('ghost.deadCount', { count: dead.length }) }}</span>
          </label>
          <span class="group-note">{{ $t('ghost.deadHint') }}</span>
        </div>

        <ul class="ghost-list">
          <li v-for="item in dead" :key="item.path" class="ghost-row">
            <label class="row-main">
              <input
                type="checkbox"
                :checked="selectedDead.has(item.path)"
                @change="toggle(selectedDead, item.path)"
              />
              <span class="row-text">
                <span class="row-name">{{ fileName(item.path) }}</span>
                <span class="row-dir">{{ dirName(item.path) }}</span>
              </span>
            </label>
            <span class="row-meta">
              <span class="badge">{{ $t('ghost.frameCount', { count: item.frame_count }) }}</span>
              <span v-if="item.exists_on_disk" class="tag tag-back">{{ $t('ghost.reappeared') }}</span>
              <span class="dim">{{ $t('ghost.markedAt', { value: item.dead_at ?? '—' }) }}</span>
            </span>
          </li>
        </ul>

        <div class="group-actions">
          <button
            class="btn btn-primary"
            :disabled="props.busy || selectedDead.size === 0"
            @click="apply('restore', selectedDead)"
          >
            {{ $t('ghost.restore') }}
          </button>
          <button
            class="btn btn-danger"
            :disabled="props.busy || selectedDead.size === 0"
            @click="apply('purge', selectedDead)"
          >
            {{ $t('ghost.purge') }}
          </button>
          <span v-if="selectedDead.size" class="sel-count">
            {{ $t('ghost.selectedDead', { count: selectedDead.size }) }}
            <template v-if="backOnDisk.filter((i) => selectedDead.has(i.path)).length">
              {{ $t('ghost.restoredAmong', { count: backOnDisk.filter((i) => selectedDead.has(i.path)).length }) }}
            </template>
          </span>
        </div>
      </section>

      <footer class="ghost-foot">
        <span v-if="props.message" class="foot-msg">{{ props.message }}</span>
        <span v-else-if="props.busy" class="dim">{{ $t('common.processing') }}</span>
        <button class="btn btn-secondary" @click="emit('close')">{{ $t('common.close') }}</button>
      </footer>

      <!-- P0-4：purge 终态删除的二次确认（应用内，替代 window.confirm） -->
      <ConfirmDialog
        :visible="confirmOpen"
        :title="$t('ghost.purgeConfirmTitle')"
        :message="$t('ghost.purgeConfirmMessage', { count: pendingPurge?.paths.length ?? 0, rows: pendingPurge?.rows ?? 0 })"
        :detail="$t('ghost.purgeConfirmDetail')"
        :confirm-text="$t('ghost.purge')"
        danger
        @confirm="onConfirmPurge"
        @cancel="confirmOpen = false"
      />
    </div>
  </div>
</template>

<style scoped>
.ghost-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.62);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
  padding: 24px;
}

.ghost-panel {
  width: min(920px, 100%);
  max-height: 86vh;
  overflow-y: auto;
  background: #16161d;
  border: 1px solid #333344;
  border-radius: 10px;
  padding: 20px 22px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.ghost-head h2 {
  margin: 0 0 6px;
  font-size: 18px;
  color: #fff;
}
.ghost-hint {
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

.ghost-empty {
  padding: 22px;
  text-align: center;
  font-size: 14px;
  color: #9a9aae;
  background: #121218;
  border: 1px dashed #333344;
  border-radius: 8px;
}

.ghost-group {
  background: #121218;
  border: 1px solid #2a2a36;
  border-radius: 8px;
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

/* 离线盘分组：灰显 + 危险色提示，不可操作 */
.ghost-group-offline {
  border-color: rgba(255, 170, 51, 0.35);
}
.offline-title {
  color: #ffaa33;
}
.ghost-row-disabled {
  opacity: 0.55;
}
.offline-note {
  font-size: 12px;
  color: #ffaa33;
  padding: 8px 10px;
  background: rgba(255, 170, 51, 0.06);
  border: 1px dashed rgba(255, 170, 51, 0.25);
  border-radius: 6px;
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

.ghost-list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 260px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.ghost-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 7px 8px;
  border-radius: 6px;
}
.ghost-row:hover {
  background: #1c1c26;
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
.tag-back {
  font-size: 11px;
  padding: 2px 7px;
  border-radius: 10px;
  background: #1e3a2a;
  color: #7fd6a0;
}
.dim {
  font-size: 11px;
  color: #6f6f85;
}

.group-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
  padding-top: 4px;
  border-top: 1px solid #26262f;
}
.sel-count {
  font-size: 12px;
  color: #8a8a9c;
  margin-left: auto;
}

.ghost-foot {
  display: flex;
  align-items: center;
  justify-content: flex-end;
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
.btn-danger {
  background: #5a2028;
  border-color: #7a2b35;
}
.btn-danger:hover:not(:disabled) {
  background: #6d2731;
}
</style>
