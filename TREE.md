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
│   └── download_models.py          # Model download helper
└── src/
    ├── frontend/                   # Vue 3 + Tauri desktop app
    │   ├── src/
    │   │   ├── App.vue
    │   │   ├── main.ts
    │   │   └── vite-env.d.ts
    │   ├── public/
    │   │   ├── tauri.svg
    │   │   └── vite.svg
    │   ├── .gitignore
    │   ├── package.json
    │   ├── package-lock.json
    │   ├── vite.config.ts
    │   ├── tsconfig.json
    │   ├── tsconfig.node.json
    │   └── src-tauri/              # Rust core (Tauri backend)
    │       ├── bin/                # Placeholder for packaged ai_worker (generated)
    │       ├── capabilities/default.json
    │       ├── src/
    │       │   ├── assets/
    │       │   │   └── vue.svg
    │       │   ├── main.rs
    │       │   └── lib.rs
    │       ├── build.rs
    │       ├── Cargo.toml
    │       ├── Cargo.lock
    │       ├── tauri.conf.json
    │       └── .gitignore
    ├── proto/
    │   └── search.proto            # Inter-process communication schema
    └── inference-worker/           # Python AI inference engine
        ├── main.py
        ├── search_pb2.py
        ├── requirements.txt
        ├── README.md               # Worker-specific documentation
        └── models/                 # Created by download_models.py (not tracked in git)
            ├── siglip2-base/
            └── easyocr/

```

> **Note:** The `models/` directory under `src/inference-worker/` is created by `download_models.py` and is not tracked in Git. The `bin/` directory under `src/frontend/src-tauri/` is for the packaged `ai_worker` executable (also not tracked).
