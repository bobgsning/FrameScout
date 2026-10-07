# FrameScout Frontend

The Vue 3 + TypeScript single-page application that powers the FrameScout desktop UI. It talks to the Rust/Tauri core over Tauri IPC and renders the search console, result grid, clusters, lightbox, and settings.

## Layered Architecture

The frontend is organized into five layers, from pure data types down to UI components. `App.vue` is only a thin orchestrator that wires the modules together.

```text
src/FrameScout-UI/src/
│
├── types/                        # 1. Pure TypeScript domain types
│   ├── search.ts                 # SearchResult, ClusterGroup, SmartFolder, PagedResponse, ...
│   └── license.ts                # LicenseStatus, EngineStatusPayload, ScanProgressPayload, ...
│
├── utils/                        # 2. Stateless pure helpers
│   ├── score.ts                  # mapToHumanScore, formatScore, getNumericScore, LOW_SCORE_THRESHOLD
│   ├── highlight.ts              # highlight() — HTML-escape + regex-metachar escaping
│   ├── media.ts                  # isVideo, getAssetUrl (convertFileSrc wrapper), withFrameAnchor
│   ├── videoTimers.ts            # video load-timeout registry (module-level singleton)
│   ├── exporters.ts              # exportCSV / exportMarkdown / exportJSON
│   └── api.ts                    # saveNote, runOcrForItem, parseLanguages
│
├── styles/                       # 3. Global styles & theme
│   ├── variables.css             # brand gradients, Pro/Trial theme variables
│   ├── animations.css            # spin, pulse, slideDown, fadeIn, scaleIn
│   └── common.css                # buttons, forms, cards, tags, panels
│
├── composables/                  # 4. State + backend IPC logic slices
│   ├── useEngineStatus.ts        # 'engine-status' events, reconnect, readiness
│   ├── useLicense.ts             # license status, activation flow, Pro verification
│   ├── useScanner.ts             # folder/file scanning, 'scan-progress' bus, stop/ETA
│   ├── useSearch.ts              # multi-modal search, image search, pagination, race-guard
│   ├── useSmartFolders.ts        # smart-folder CRUD, rename, count refresh
│   ├── useClustering.ts          # cluster threshold + request
│   ├── useTextEntries.ts         # imported text-entries state & actions
│   ├── useToast.ts               # global toast queue + error localization
│   ├── useKeyboard.ts            # Esc / `/` / arrows / Space global shortcuts
│   ├── useFileActions.ts         # open file / reveal in explorer / copy path
│   ├── usePreferences.ts         # persisted prefs (library path, page size, OCR lang, history)
│   └── useSelection.ts           # multi-select candidate set
│
└── components/                   # 5. Views & interactive components
    ├── common/SvgDefs.vue        # shared hidden SVG defs, gradients, logo symbols
    ├── splash/SplashScreen.vue   # startup / model-loading screen
    ├── header/BrandHeader.vue    # logo, title, license pill
    ├── header/LicenseModal.vue   # Pro license activation dialog
    ├── scan/TopActionBar.vue     # path input, Browse, scan config, OCR toggle, ghost cleanup
    ├── scan/ExtractionBus.vue    # real-time extraction progress
    ├── scan/IncomingBanner.vue   # floating "new files indexed" banner
    ├── scan/GhostCleanupDialog.vue   # read-only preview + purge (offline-drive guard)
    ├── scan/DiffReportDialog.vue     # change-perception report (sense-only, no action)
    ├── scan/ScanReportDialog.vue     # scan summary / failures
    ├── smart-folders/SmartFolderBar.vue
    ├── cluster/ClusterView.vue   # visual similarity grid + threshold slider
    ├── search/SearchConsole.vue  # query input, channel filters, visual-target banner
    ├── search/PaginationBar.vue  # paging, Jump, Show All toggle
    ├── search/TextEntryDialog.vue
    ├── search/TextEntryManagerDialog.vue
    ├── cards/ResultCard.vue      # dispatches Video/Image/Missing; context menu + double-click
    ├── cards/VideoCard.vue       # video player with timestamp seek + load-timeout handling
    ├── cards/ImageCard.vue       # image card with low-confidence fold accordion
    ├── cards/ScoreBar.vue        # per-channel score-source bars
    ├── cards/OcrPanel.vue        # extracted OCR text, expand/copy, on-demand recognition
    ├── cards/NotePanel.vue       # Markdown notes (preview/edit, frame timestamp, save feedback)
    ├── cards/TextEntryCard.vue   # plain-text entry card
    ├── Lightbox.vue              # fullscreen preview (zoom / rotate / pan / OCR red-box)
    ├── SelectionTray.vue         # candidate set + CSV/Markdown/JSON export
    ├── SettingsDialog.vue        # settings + backup + integrity check
    ├── TimelineView.vue          # data timeline + "on this day last year"
    ├── ContextMenu.vue           # right-click menu
    ├── ToastContainer.vue        # global toast notifications
    ├── ConfirmDialog.vue         # in-app confirm dialog (replaces native window.confirm)
    └── PromptDialog.vue          # inline text-input dialog (replaces native prompt)

    └── App.vue                   # thin orchestrator (~770 lines: template + script glue)
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

## Rust/Tauri Command Contract

The frontend calls these Tauri commands (all defined in `src-tauri/src/commands/`):

```text
get_license_status          → LicenseStatus
activate_pro_license        → string (email, licenseKey)
scan_folder                 → number (folderPath, scanMode, enableOcr, ocrLanguages[])
cancel_scan                 → void
index_files                 → number (filePaths[], enableOcr, ocrLanguages[])
preview_ghosts              → GhostItem[]            (read-only; offline-drive guard)
apply_ghost_action          → GhostActionResult (paths[], action: mark_dead | restore | purge)
run_ocr_for_selected_files  → number (filePaths[], languages[])
update_note                 → void (path, note, timestamp?)  (timestamp = video frame)
list_all_files              → { items, total_count } (page, limit)
search_images               → { items, total_count } (text, page, limit, useVector, useOcr, useNote, useFilename)
search_by_image             → { items, total_count } (imagePath, page, limit)
search_text                 → { items, total_count } (BGE-M3 text semantic)
search_unified              → { items, total_count } (cross-channel fusion; matches + ocr_lines)
get_all_files               → SearchResult[]
cluster_similar_images      → ClusterResult (threshold)   (medoid + sampling + truncation notice)
get_smart_folders           → SmartFolder[]
save_smart_folder           → void (name, queryText, useVector, useOcr, useNote, useFilename)
update_smart_folder         → void (rename / update query)
refresh_smart_folder_count  → number (id)
delete_smart_folder         → void (id)
execute_smart_folder        → { items, total_count } (id, page, limit)
ingest_text_files           → number (chunked text ingestion)
list_timeline               → TimelineDay[]
last_year_today_count       → number
backup_database             → string (timestamped .db copy, rotates 3)
verify_integrity            → string (PRAGMA integrity_check)
list_backups                → BackupInfo[]
delete_backup               → void (path)
probe_ffmpeg                → boolean
cut_video_clip              → string (inputPath, outputPath, startSec, durationSec?)
```

Backend events listened to:

```text
engine-status   → { status: 'connecting' | 'ready' | 'error', retry?, max_retries?, message }
scan-progress   → { status, file_path, current, total, new_files? }
```

## Development Notes

- **Path alias**: `@/` resolves to `src/` (configured in `vite.config.ts` + `tsconfig.json`).
- **Asset URLs**: images/videos go through `getAssetUrl()` + Tauri `convertFileSrc` (`asset://` protocol) — never concatenate raw file paths.
- **Global styles**: imported in `main.ts` (variables → animations → common, in order), because scoped `<style>` hashes `@keyframes` and `.highlight-text` is injected via `v-html`.
