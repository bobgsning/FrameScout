# TREE

本文件是 FrameScout 仓库的目录导览，与代码结构保持同步。
详细模块说明请阅读各目录下的 README：

| 目录 | 文档 |
| ---- | ---- |
| `src/inference-worker/` | [README.md](./src/inference-worker/README.md) · [BUILD.md](./src/inference-worker/BUILD.md) · [OCR_MULTILANG.md](./src/inference-worker/OCR_MULTILANG.md) |
| `src/FrameScout-UI/src/`（前端） | [README.md](./src/FrameScout-UI/src/README.md) |
| `src/FrameScout-UI/src-tauri/src/`（Rust 核心） | [README.md](./src/FrameScout-UI/src-tauri/src/README.md) |

```text
FrameScout/
├── .gitignore
├── AI_POLICY.md                # AI 辅助开发政策
├── CHANGELOG.md                # 更新日志（当前发布版 v3.3.0 — 2026-10-18）
├── CONTRIBUTING.md             # 贡献指南
├── COPYING.LGPLv2.1            # LGPL v2.1 完整文本（随包分发的 FFmpeg，见 THIRD_PARTY_NOTICES.md）
├── LICENSE                     # Apache License 2.0（项目本体）
├── README.md                   # 英文主页
├── README.zh-CN.md             # 中文主页
├── THIRD_PARTY_NOTICES.md      # 第三方组件与模型权重声明
├── TREE.md
├── pyrightconfig.json
├── requirements.txt
├── scripts/
│   ├── download_models.py           # SigLIP 2 + BGE-M3 模型下载 / ONNX 导出助手
│   ├── download_ocr_models.py       # OCR 多语言 rec 模型下载（内置随包 + 可按需全量）
│   └── check_i18n_backend.mjs       # 9 语 i18n 键 / 占位符一致性校验脚本
└── src/
    ├── FrameScout-UI/              # Vue 3 + Tauri 桌面应用（Rust 核心）
    │   ├── src/
    │   │   ├── App.vue             # Top-level orchestrator (UI composition only)
    │   │   ├── main.ts             # App bootstrap + global styles + i18n locale init
    │   │   ├── env.d.ts
    │   │   ├── components/         # Vue components
    │   │   │   ├── cards/          # ResultCard / ImageCard / VideoCard / ScoreBar / OcrPanel / NotePanel / TextEntryCard
    │   │   │   ├── cluster/        # ClusterView
    │   │   │   ├── common/         # SvgDefs (shared SVG assets)
    │   │   │   ├── header/         # BrandHeader / LicenseModal
    │   │   │   ├── scan/           # TopActionBar / ExtractionBus / IncomingBanner / GhostCleanupDialog / DiffReportDialog / ScanReportDialog
    │   │   │   ├── search/         # SearchConsole / PaginationBar / TextEntryDialog / TextEntryManagerDialog / ConditionRow
    │   │   │   ├── smart-folders/  # SmartFolderBar
    │   │   │   ├── splash/         # SplashScreen
    │   │   │   ├── Lightbox.vue    # fullscreen preview (zoom / rotate / OCR red-box)
    │   │   │   ├── SelectionTray.vue   # multi-select candidate set + export
    │   │   │   ├── SettingsDialog.vue  # settings + backup/restore + integrity check + cleanup policy
    │   │   │   ├── TimelineView.vue    # data timeline + "on this day last year"
    │   │   │   ├── ConfirmDialog.vue   # in-app confirm dialog
    │   │   │   ├── ContextMenu.vue     # right-click menu
    │   │   │   ├── ToastContainer.vue  # global toast notifications
    │   │   │   ├── PromptDialog.vue    # inline text-input dialog
    │   │   │   ├── ClipDialog.vue     # video clip cutting dialog (cut_video_clip)
    │   │   │   └── OcrLanguageList.vue # OCR language tiers + download/delete UI
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
    │   │   │   ├── useTextEntries.ts
    │   │   │   ├── useOcrLanguages.ts   # OCR language catalog / download / delete state
    │   │   │   └── useClip.ts           # video clip-cutting state machine
    │   │   ├── i18n/               # vue-i18n: 9 locales + backend error-code localization
    │   │   │   ├── index.ts        # i18n instance, setLocale, localizeCode
    │   │   │   └── locales/        # en / zh-CN / zh-TW / ja / ru / ko / de / fr / es (.json)
    │   │   ├── types/              # TypeScript domain types (search / license / ocr)
    │   │   ├── utils/              # Pure helpers (api / highlight / media / score / videoTimers / exporters / backendText)
    │   │   └── styles/             # Global CSS (variables / animations / common)
    │   ├── src-tauri/              # Rust / Tauri backend
    │   │   ├── capabilities/
    │   │   │   └── main.json      # Tauri 权限/能力集
    │   │   ├── src/
    │   │   │   ├── main.rs         # Desktop entry point
    │   │   │   ├── lib.rs          # Library entry (Tauri setup + command registration)
    │   │   │   ├── constants.rs    # Global constants (VECTOR_DIM, FREE_TRIAL_LIMIT)
    │   │   │   ├── proto.rs        # Generated protobuf include
    │   │   │   ├── error.rs        # Centralized error/status/tag codes (i18n contract)
    │   │   │   ├── logging.rs      # fs_trace! trace-level logging macro
    │   │   │   ├── time_util.rs    # Lightweight UTC stamp helper (backup names / batch ids)
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
    │   │   │   │   ├── license_cmd.rs
    │   │   │   │   ├── export_cmd.rs        # export_file (write to user-configured dir)
    │   │   │   │   ├── model_cmd.rs         # OCR model list / download / delete
    │   │   │   │   └── settings_cmd.rs      # cleanup policy (retention / backup rotation)
    │   │   │   ├── services/       # worker_process / zmq_client / engine_monitor / paths
    │   │   │   ├── storage/        # db / vector_matrix / vector_blob / migrations / journal / path_util / text_store / ocr_store / app_settings
    │   │   │   ├── model_code/     # Rust data models / AppState (state / search / smart_folder / cluster / progress / frame / ghost / diff / report / text)
    │   │   │   └── license/        # verifier / guard
    │   │   ├── capabilities/
    │   │   │   └── main.json      # Tauri 权限/能力集
    │   │   ├── Cargo.toml
    │   │   ├── tauri.conf.json
    │   │   ├── pyrightconfig.json
    │   │   └── bin/               # 打包产物落点：build_nuitka.ps1 部署 ai_worker 于此（git-ignored，仅本地存在）
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
        ├── pyrightconfig.json       # Worker-side type-check config
        ├── ocr_manifest.json       # OCR language catalog single source of truth
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
            ├── ocr/                # OCR rec models: ch_tra / ja / en_opt / ru / ko / de / fr / es
            └── bge-m3/             # BGE-M3 dense text embeddings
```

> **Note:** `src/FrameScout-UI/src-tauri/{bin,target}` and `src/inference-worker/{build,dist,dist_nuitka,.ffmpeg-cache}`
> are build/package artifacts (Nuitka / PyInstaller / Cargo / Rust / FFmpeg download cache) and are **not**
> tracked in Git. The Python source for the worker lives in `main.py`, `server.py`, `config.py`, `engines/`,
> `utils/`, and `media/`.
