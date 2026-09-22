# TREE

```text

FrameScout/
├── .gitignore
├── AI_POLICY.md
├── CHANGELOG.md
├── CONTRIBUTING.md
├── LICENSE
├── README.md
├── TREE.md
├── scripts/
│   └── download_models.py          # Model download & ONNX export helper
└── src/
    ├── FrameScout-UI/              # Vue 3 + Tauri desktop app (Rust core)
    │   ├── src/
    │   │   ├── App.vue             # Top-level orchestrator (UI composition only)
    │   │   ├── main.ts             # App bootstrap + global styles
    │   │   ├── components/         # Vue components
    │   │   │   ├── cards/          # ResultCard / ImageCard / VideoCard / OcrPanel / NotePanel
    │   │   │   ├── cluster/        # ClusterView
    │   │   │   ├── common/         # SvgDefs (shared SVG assets)
    │   │   │   ├── header/         # BrandHeader / LicenseModal
    │   │   │   ├── scan/           # TopActionBar / ExtractionBus / IncomingBanner
    │   │   │   ├── search/         # SearchConsole / PaginationBar
    │   │   │   ├── smart-folders/  # SmartFolderBar
    │   │   │   └── splash/         # SplashScreen
    │   │   ├── composables/        # Vue 3 logic slices (state + Tauri IPC)
    │   │   │   ├── useEngineStatus.ts
    │   │   │   ├── useLicense.ts
    │   │   │   ├── useSmartFolders.ts
    │   │   │   ├── useScanner.ts
    │   │   │   ├── useSearch.ts
    │   │   │   └── useClustering.ts
    │   │   ├── types/              # TypeScript domain types (search / license)
    │   │   ├── utils/              # Pure helpers (api / highlight / media / score / videoTimers)
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
    │   │   │   │   ├── ocr_cmd.rs
    │   │   │   │   ├── cluster_cmd.rs
    │   │   │   │   ├── smart_folder_cmd.rs
    │   │   │   │   ├── file_cmd.rs
    │   │   │   │   └── license_cmd.rs
    │   │   │   ├── services/       # worker_process / zmq_client / engine_monitor
    │   │   │   ├── storage/        # db / vector_matrix
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
    │   ├── search.proto            # Inter-process communication schema (Rust ⇄ Python)
    │   └── protoc.exe             # Bundled protobuf compiler (offline)
    └── inference-worker/           # Python AI inference engine
        ├── main.py                 # Minimal bootstrap entry
        ├── server.py               # ZeroMQ dispatch center
        ├── config.py               # Global config & path resolution
        ├── license_gen.py          # Pro license key generator (dev only)
        ├── search_pb2.py           # Generated protobuf (from search.proto)
        ├── requirements.txt
        ├── ai_worker.spec          # PyInstaller spec
        ├── engines/                # ocr_engine / siglip_engine
        ├── utils/                  # logger / onnx_utils / vector_utils
        ├── media/                  # image_io / video_extractor
        └── models/                 # Created by download_models.py (git-ignored)
            ├── siglip2-base/
            └── easyocr/
```

> **Note:** `src/FrameScout-UI/src-tauri/{bin,target}` and `src/inference-worker/{build,dist}` are build/package
> artifacts (PyInstaller / Cargo / Rust) and are **not** tracked in Git. The Python source for the worker
> lives in `main.py`, `server.py`, `config.py`, `engines/`, `utils/`, and `media/`.
