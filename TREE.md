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
    │   ├── App.vue
    │   ├── package.json
    │   ├── package-lock.json
    │   ├── vite.config.ts
    │   ├── tsconfig.json
    │   ├── tsconfig.node.json
    │   └── src-tauri/              # Rust core (Tauri backend)
    │       ├── capabilities/default.json
    │       ├── src/
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
        └── README.md               # Worker-specific documentation

```
