# FrameScout Frontend

The Vue 3 + TypeScript single-page application that powers the FrameScout desktop UI. It talks to the Rust/Tauri core over Tauri IPC and renders the search console, result grid, clusters, lightbox, and settings.

## Layered Architecture

The frontend is organized into six layers, from pure data types down to UI components. `App.vue` is only a thin orchestrator that wires the modules together.

```text
src/FrameScout-UI/src/
│
├── types/                        # 1. Pure TypeScript domain types
│   ├── search.ts                 # SearchResult, ClusterGroup, SmartFolder, PagedResponse, ...
│   ├── license.ts                # LicenseStatus, EngineStatusPayload, ScanProgressPayload, ...
│   └── ocr.ts                    # OcrLanguageInfo, OcrDownloadProgress, OcrTier
│
├── utils/                        # 2. Stateless pure helpers
│   ├── score.ts                  # mapToHumanScore, mapBgeScore, formatScore, getNumericScore,
│   │                             #   getLowScoreThreshold, SEMANTIC_BADGE_THRESHOLD, isLiteralOnlyHit, ...
│   ├── highlight.ts              # highlight() — HTML-escape + regex-metachar escaping
│   ├── media.ts                  # isVideo, getAssetUrl (convertFileSrc wrapper), withFrameAnchor, clipOutputPath
│   ├── videoTimers.ts            # video load-timeout registry (module-level singleton)
│   ├── exporters.ts              # exportCSV / exportMarkdown / exportJSON (export_file first, browser download fallback)
│   ├── backendText.ts            # localizeBackend — forwards to i18n localizeCode (backend error codes)
│   └── api.ts                    # saveNote, runOcrForItem
│
├── i18n/                         # 3. Localization (vue-i18n, 9 locales)
│   ├── index.ts                  # i18n instance, SUPPORTED_LOCALES, setLocale, localizeCode
│   └── locales/                  # en / zh-CN / zh-TW / ja / ru / ko / de / fr / es (.json)
│
├── styles/                       # 4. Global styles & theme
│   ├── variables.css             # brand gradients, Pro/Trial theme variables
│   ├── animations.css            # spin, pulse, slideDown, fadeIn, scaleIn
│   └── common.css                # buttons, forms, cards, tags, panels
│
├── composables/                  # 5. State + backend IPC logic slices
│   ├── useEngineStatus.ts        # 'engine-status' events, reconnect, readiness
│   ├── useLicense.ts             # license status, activation flow, Pro verification
│   ├── useScanner.ts             # folder/file scanning, 'scan-progress' bus, stop/pause/resume/ETA
│   ├── useSearch.ts              # multi-modal search, image search, pagination, race-guard
│   ├── useSmartFolders.ts        # smart-folder CRUD, rename, count refresh, criteria rows
│   ├── useClustering.ts          # cluster threshold + request
│   ├── useTextEntries.ts         # imported text-entries state & actions
│   ├── useToast.ts               # global toast queue + error localization
│   ├── useKeyboard.ts            # Esc / `/` / arrows / Space global shortcuts
│   ├── useFileActions.ts         # open file / reveal in explorer / copy path
│   ├── usePreferences.ts         # persisted prefs (library path, page size, OCR lang, export dir, history)
│   ├── useSelection.ts           # multi-select candidate set
│   ├── useOcrLanguages.ts        # OCR language catalog (3 tiers), download with progress, delete
│   └── useClip.ts                # video clip-cutting state machine (pendingItem + duration)
│
└── components/                   # 6. Views & interactive components
    ├── common/SvgDefs.vue        # shared hidden SVG defs, gradients, logo symbols
    ├── splash/SplashScreen.vue   # startup / model-loading screen
    ├── header/BrandHeader.vue    # logo, title, license pill
    ├── header/LicenseModal.vue   # Pro license activation dialog
    ├── scan/TopActionBar.vue     # path input, Browse, scan config, OCR toggle, ghost cleanup
    ├── scan/ExtractionBus.vue    # real-time extraction progress
    ├── scan/IncomingBanner.vue   # floating "new files indexed" banner
    ├── scan/GhostCleanupDialog.vue   # read-only preview + purge (offline-drive guard)
    ├── scan/DiffReportDialog.vue     # change-perception report (sense-only, no action)
    ├── scan/ScanReportDialog.vue     # scan summary / failures / batch filter
    ├── smart-folders/SmartFolderBar.vue
    ├── cluster/ClusterView.vue   # visual similarity grid + threshold slider
    ├── search/SearchConsole.vue  # query input, channel filters, visual-target banner
    ├── search/PaginationBar.vue  # paging, Jump, Show All toggle
    ├── search/TextEntryDialog.vue
    ├── search/TextEntryManagerDialog.vue
    ├── search/ConditionRow.vue   # one search/smart-folder condition row (keyword / date / batch / seed image)
    ├── cards/ResultCard.vue      # dispatches Video/Image/Missing; context menu + double-click
    ├── cards/VideoCard.vue       # video player with timestamp seek + load-timeout handling
    ├── cards/ImageCard.vue       # image card with low-confidence fold accordion
    ├── cards/ScoreBar.vue        # per-channel score-source bars (literal hits shown as ✓, not fake %)
    ├── cards/OcrPanel.vue        # extracted OCR text, expand/copy, on-demand recognition
    ├── cards/NotePanel.vue       # Markdown notes (preview/edit, frame timestamp, save feedback)
    ├── cards/TextEntryCard.vue   # plain-text entry card
    ├── Lightbox.vue              # fullscreen preview (zoom / rotate / pan / OCR red-box)
    ├── SelectionTray.vue         # candidate set + CSV/Markdown/JSON export
    ├── SettingsDialog.vue        # settings + backup/restore + integrity check + cleanup policy + OCR languages
    ├── TimelineView.vue          # data timeline + "on this day last year"
    ├── ContextMenu.vue           # right-click menu
    ├── ToastContainer.vue        # global toast notifications
    ├── ConfirmDialog.vue         # in-app confirm dialog (replaces native window.confirm)
    ├── PromptDialog.vue          # inline text-input dialog (replaces native prompt)
    ├── ClipDialog.vue            # video clip cutting (duration + output dir)
    └── OcrLanguageList.vue       # OCR language tiers with download progress / delete

    └── App.vue                   # thin orchestrator (~803 lines: template + script glue)
```

## Key Architectural Decisions

### Module-level singletons instead of Pinia

State is declared as module-scoped `ref`s outside the exported `useXxx()` functions, so every component that calls `useSearch()`, `useScanner()`, etc. receives the same shared state — no Pinia or `provide`/`inject` required.

```ts
// useSearch.ts
const searchQuery = ref('')        // module scope, not inside the function
export function useSearch() { /* ... */ }
```

### Lifecycle guards against duplicate subscriptions

Composables like `useScanner()` are called from multiple components. Without a guard, `listen('scan-progress')` would register twice and process each event twice. Every effectful composable guards its lifecycle hooks:

```ts
let lifecycleBound = false
export function useScanner() {
  if (!lifecycleBound) {
    lifecycleBound = true
    onMounted(/* register once */)
    onUnmounted(/* ... */)
  }
  return { /* ... */ }
}
```

### Video timeout registry

`utils/videoTimers.ts` is a module-level singleton: `useSearch` arms a timeout when a search returns video results, `ResultCard` clears it on load success/failure, and `clearAllVideoTimers()` runs on unmount.

### Backend error codes → localized text

The Rust core never sends English sentences to the UI — it returns stable codes (`CODE` or `CODE|param0|param1`, see `src-tauri/src/error.rs`). `i18n/index.ts::localizeCode()` splits on `|`, looks the code up in `backend.*` of the active locale, and interpolates `{0}`/`{1}`/`{detail}`. `utils/backendText.ts` only forwards to it (one implementation, no drift). Non-code content (real paths, plain sentences) passes through unchanged, so it is safe for fields like `scan-progress.file_path` that sometimes carry a path and sometimes a status. `t()` also runs its `{err}` template parameter through `localizeCode`, so ~50 `t('...failed', { err })` call sites localize for free. Unknown codes fall back to the raw string — a bare key is never shown to the user.

## Rust/Tauri Command Contract

The frontend calls these Tauri commands (all defined in `src-tauri/src/commands/`, registered in `lib.rs`):

```text
get_license_status          → LicenseStatus
activate_pro_license        → string (email, licenseKey)
scan_folder                 → number (folderPath, scanMode, enableOcr, ocrLanguages[])
cancel_scan                 → void
pause_scan                  → void
resume_scan                 → void
index_files                 → number (filePaths[], enableOcr, ocrLanguages[])
run_ocr_for_selected_files  → number (filePaths[], languages[])
preview_ghosts              → GhostItem[]            (read-only; offline-drive guard)
apply_ghost_action          → GhostActionResult (paths[], action: mark_dead | restore | purge)
scan_diff                   → DiffReport             (added / modified / missing / no-vector)
reindex_files               → number (filePaths[])
list_scan_reports           → BatchReport[]          (batch id / type filter)
list_timeline               → TimelineDay[]
list_files_by_day           → SearchResult[]         (day)
last_year_today_count       → number
update_note                 → void (path, note, timestamp?)  (timestamp = video frame)
list_all_files              → { items, total_count } (page, limit)
get_all_files               → SearchResult[]
search_by_image             → { items, total_count } (imagePath, page, limit, batchId?, rootPath?, indexTimeFrom?, indexTimeTo?, fileTypes?)
search_text                 → { items, total_count } (BGE-M3 text semantic)
ping_engine                 → boolean
insert_text_entry           → void (path)
search_text_entries         → { items, total_count } (query, page, limit)
search_unified              → { items, total_count } (cross-channel fusion; matches + ocr_lines; batchId?/rootPath?/indexTimeFrom?/indexTimeTo?/fileTypes?)
cluster_similar_images      → ClusterResult (threshold)   (medoid + sampling + truncation notice)
get_smart_folders           → SmartFolder[]   (含 criteria_json + needs_frontend_eval)
save_smart_folder           → void (name, queryText, useVector, useOcr, useNote, useFilename, criteriaJson?)
update_smart_folder         → void (rename / update query / criteriaJson?)
refresh_smart_folder_count  → number (id)     (条件行文件夹返回 0，前端显示 "?")
delete_smart_folder         → void (id)
import_smart_folders        → { imported, renamed, names } (folders[]；同名自动加序号后缀)
ingest_text_files           → number (chunked text ingestion)
preview_text_ingest         → { will_replace_chunks, source_count } (真预演，不用全库总数冒充)
list_text_entries           → TextEntryPage
delete_text_entries         → number (paths[])
backup_database             → string (timestamped .db copy, rotates 3)
restore_backup              → RestoreResult (path)   (写 pending_restore 标记，下次启动生效)
preview_restore             → RestorePreview         (恢复前的合法性与影响预览)
verify_integrity            → string (PRAGMA integrity_check)
list_backups                → BackupInfo[]
delete_backup               → void (path)
probe_ffmpeg                → boolean
cut_video_clip              → string (inputPath, outputPath, startSec, durationSec?)
export_file                 → string (dir, filename, content)  (空 dir 时前端回退浏览器下载)
list_ocr_languages          → OcrLanguageInfo[]      (三档目录，来自 ocr_manifest.json)
download_ocr_model          → void (code)            (Range 续传 + SHA256 + 磁盘检查)
delete_ocr_model            → void (code)
get_cleanup_settings        → CleanupSettings        (事件保留天数 / 备份份数 / 备份天数；-1 = 永不)
set_cleanup_settings        → void (settings)
get_startup_cleanup_report  → StartupCleanupReport   (启动期自动清理汇总，Toast 告知)
```

Backend events listened to:

```text
engine-status            → { status: 'connecting' | 'ready' | 'error', retry?, max_retries?, message }
scan-progress            → { status, file_path, current, total, new_files? }
ocr-download-progress    → { language, downloaded, total, percent }
```

## Development Notes

- **Path alias**: `@/` resolves to `src/` (configured in `vite.config.ts` + `tsconfig.json`).
- **Asset URLs**: images/videos go through `getAssetUrl()` + Tauri `convertFileSrc` (`asset://` protocol) — never concatenate raw file paths.
- **Global styles**: imported in `main.ts` (variables → animations → common, in order), because scoped `<style>` hashes `@keyframes` and `.highlight-text` is injected via `v-html`.
- **i18n bootstrap**: `main.ts` calls `setLocale()` from persisted preferences (default `en`) and watches for changes. `scripts/check_i18n_backend.mjs` verifies that all 9 locale files carry the same `backend.*` key set and `{0}/{1}/{detail}` placeholders as `en`.
- **Score semantics**: semantic channels are cosine similarities mapped to a human 0–100 scale; literal channels (OCR / note / filename) are boolean hits rendered as ✓ — never fabricate a percentage for them.
