# TREE

```text

FrameScout/
├── .gitignore
├── AI_POLICY.md
├── CHANGELOG.md
├── CONTRIBUTING.md
├── LICENSE
├── README.md
├── README.zh-CN.md
├── TREE.md
├── scripts/
│   └── download_models.py          # Model download & ONNX export helper
└── src/
    ├── FrameScout-UI/              # Vue 3 + Tauri desktop app (Rust core)
    │   ├── src/
    │   │   ├── App.vue             # Top-level orchestrator (UI composition only)
    │   │   ├── main.ts             # App bootstrap + global styles
    │   │   ├── components/         # Vue components
    │   │   │   ├── cards/          # ResultCard / ImageCard / VideoCard / ScoreBar / OcrPanel / NotePanel / TextEntryCard
    │   │   │   ├── cluster/        # ClusterView
    │   │   │   ├── common/         # SvgDefs (shared SVG assets)
    │   │   │   ├── header/         # BrandHeader / LicenseModal
    │   │   │   ├── scan/           # TopActionBar / ExtractionBus / IncomingBanner / GhostCleanupDialog / DiffReportDialog / ScanReportDialog
    │   │   │   ├── search/         # SearchConsole / PaginationBar / TextEntryDialog / TextEntryManagerDialog
    │   │   │   ├── smart-folders/  # SmartFolderBar
    │   │   │   ├── splash/         # SplashScreen
    │   │   │   ├── Lightbox.vue    # fullscreen preview (zoom / rotate / OCR red-box)
    │   │   │   ├── SelectionTray.vue   # multi-select candidate set + export
    │   │   │   ├── SettingsDialog.vue  # settings + backup + integrity check
    │   │   │   ├── TimelineView.vue    # data timeline + "on this day last year"
    │   │   │   ├── ConfirmDialog.vue   # in-app confirm dialog
    │   │   │   ├── ContextMenu.vue     # right-click menu
    │   │   │   ├── ToastContainer.vue  # global toast notifications
    │   │   │   └── PromptDialog.vue    # inline text-input dialog
    │   │   ├── composables/        # Vue 3 logic slices (state + Tauri IPC)
    │   │   │   ├── useEngineStatus.ts
    │   │   │   ├── useLicense.ts
    │   │   │   ├── useSmartFolders.ts
    │   │   │   ├── useScanner.ts
    │   │   │   ├── useSearch.ts
    │   │   │   ├── useClustering.ts
    │   │   │   ├── useToast.ts
    │   │   │   ├── useKeyboard.ts
    │   │   │   ├── useFileActions.ts
    │   │   │   ├── usePreferences.ts
    │   │   │   ├── useSelection.ts
    │   │   │   └── useTextEntries.ts
    │   │   ├── types/              # TypeScript domain types (search / license)
    │   │   ├── utils/              # Pure helpers (api / highlight / media / score / videoTimers / exporters)
    │   │   └── styles/            # Global CSS (variables / animations / common)
    │   ├── src-tauri/              # Rust / Tauri backend
    │   │   ├── src/
    │   │   │   ├── main.rs         # Desktop entry point
    │   │   │   ├── lib.rs          # Library entry (Tauri setup + command registration)
    │   │   │   ├── constants.rs    # Global constants (VECTOR_DIM, FREE_TRIAL_LIMIT)
    │   │   │   ├── proto.rs        # Generated protobuf include
    │   │   │   ├── commands/       # Tauri commands
    │   │   │   │   ├── index_cmd.rs
    │   │   │   │   ├── search_cmd.rs
    │   │   │   │   ├── search_unified_cmd.rs   # RRF cross-channel fusion
    │   │   │   │   ├── ocr_cmd.rs
    │   │   │   │   ├── cluster_cmd.rs
    │   │   │   │   ├── smart_folder_cmd.rs
    │   │   │   │   ├── file_cmd.rs
    │   │   │   │   ├── ghost_cmd.rs
    │   │   │   │   ├── diff_cmd.rs
    │   │   │   │   ├── reindex_cmd.rs
    │   │   │   │   ├── report_cmd.rs
    │   │   │   │   ├── text_cmd.rs
    │   │   │   │   ├── backup_cmd.rs
    │   │   │   │   ├── ffmpeg_cmd.rs
    │   │   │   │   └── license_cmd.rs
    │   │   │   ├── services/       # worker_process / zmq_client / engine_monitor
    │   │   │   ├── storage/        # db / vector_matrix / migrations / journal / path_util / text_store / ocr_store
    │   │   │   ├── model_code/         # Rust data models / AppState
    │   │   │   └── license/        # verifier / guard
    │   │   ├── capabilities/
    │   │   ├── Cargo.toml
    │   │   ├── tauri.conf.json
    │   │   └── bin/               # Placeholder for packaged ai_worker (generated, git-ignored)
    │   ├── package.json
    │   ├── vite.config.ts
    │   └── tsconfig.json
    ├── proto/
    │   ├── framescout.proto        # Inter-process communication schema (Rust ⇄ Python)
    │   └── protoc.exe             # Bundled protobuf compiler (offline)
    └── inference-worker/           # Python AI inference engine
        ├── main.py                 # Minimal bootstrap entry
        ├── server.py               # ZeroMQ dispatch center
        ├── config.py               # Global config & path resolution
        ├── framescout_pb2.py       # Generated protobuf (from framescout.proto)
        ├── framescout_pb2.pyi      # Generated protobuf stubs
        ├── requirements.txt
        ├── README.md               # Worker overview
        ├── BUILD.md                # Build & packaging guide (Nuitka)
        ├── OCR_MULTILANG.md        # OCR multi-language guide
        ├── build_nuitka.ps1        # Nuitka packaging script (preferred)
        ├── scripts/                # benchmark.py (performance baseline)
        ├── engines/                # ocr_engine / siglip_engine / bge_engine
        ├── utils/                  # logger / onnx_utils / vector_utils
        ├── media/                  # image_io / video_extractor
        └── models/                 # Created by download_models.py (git-ignored)
            ├── siglip2-base/       # SigLIP 2 ONNX (vision + text)
            └── bge-m3/             # BGE-M3 dense text embeddings
```

> **Note:** `src/FrameScout-UI/src-tauri/{bin,target}` and `src/inference-worker/{build,dist}` are build/package
> artifacts (Nuitka / PyInstaller / Cargo / Rust) and are **not** tracked in Git. The Python source for the worker
> lives in `main.py`, `server.py`, `config.py`, `engines/`, `utils/`, and `media/`.
