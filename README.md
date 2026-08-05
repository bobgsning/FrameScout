# 🔍 FrameScout

## *A Fully-Offline, Privacy-First, Multi-Modal Desktop Search Engine*

**Search your local images and videos with natural language, OCR text, or visual similarity — 100% private, zero cloud, free and open source (Community Edition).**

[![License: Apache 2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4)](https://www.microsoft.com/windows)
![Architecture](https://img.shields.io/badge/architecture-Rust%20%2B%20Python%20%2B%20Vue-ff69b4)
[![AI-Assisted](https://img.shields.io/badge/AI--assisted-development-brightgreen)](AI_POLICY.md)

---

## 🎯 The Problem

Your image library grows faster than your ability to organize it. Existing solutions either compromise privacy (cloud-based) or lack intelligent search (local tools).

FrameScout is the **first fully offline, AI-powered visual search engine** that respects your privacy and gives you instant, semantic access to your entire media library.

| Requirement | FrameScout |
| ----------- | :--------: |
| 100% Offline | ✅ |
| Semantic Search (CLIP) | ✅ |
| OCR Text Search | ✅ |
| Video Frame Indexing | ✅ |
| Visual Clustering | ✅ |
| No Telemetry | ✅ |
| Free & Open Source | ✅ |

---

## ✨ Features

| Feature | Description |
| --------- | ------------- |
| 🔒 **100% Offline** | No internet required. Models are bundled locally. Your data never leaves your computer. |
| 🧠 **Semantic Search (CLIP)** | Type *"sunset beach with friends"* and find matching images by meaning, not just filenames. |
| 🔍 **Image-to-Image Search** | Drop a reference image to find visually similar ones in your library. |
| 👁️ **OCR Text Search** | Extracts and indexes text from images. Search *"receipt from March"* and find it instantly. |
| 🎬 **Video Frame Indexing** | Automatically extracts key frames from videos and indexes them alongside static images. |
| 🧩 **Visual Clustering** | Discover groups of similar images (duplicates, near-duplicates, burst shots) with one click. |
| 📁 **Smart Folders** | Save any search as a dynamic folder that updates automatically when new files are added. |
| 📝 **Personal Notes** | Attach markdown notes to any image. Notes are searchable. |

---

## 📁 Repository Structure

```text
FrameScout/
├── .gitignore
├── AI_POLICY.md
├── CHANGELOG.md
├── CONTRIBUTING.md
├── LICENSE
├── README.md
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

---

## 🏗️ Architecture

FrameScout uses a **three-process architecture** for maximum performance, safety, and modularity:

```text
┌─────────────────────────────────────────────────────────┐
│                     Vue 3 Frontend                      │
│         (Search Console, Result Grid, Clusters)         │
└──────────────────────┬──────────────────────────────────┘
                       │  Tauri IPC
                       ▼
┌─────────────────────────────────────────────────────────┐
│                   Rust/Tauri Core                       │
│  ┌──────────┐  ┌──────────────┐  ┌────────────────┐     │
│  │ File I/O │  │ SQLite +     │  │ FlatVector     │     │
│  │ WalkDir  │  │ Rusqlite     │  │ Matrix (512D)  │     │
│  └──────────┘  └──────────────┘  └────────────────┘     │
└──────────────────────┬──────────────────────────────────┘
                       │  ZeroMQ + Protobuf (REQ/REP)
                       ▼
┌─────────────────────────────────────────────────────────┐
│                Python Inference Worker                  │
│  ┌──────────────────────────────────────────────────┐   │
│  │ CLIP ViT-B/32 (Text + Image Embeddings)          │   │
│  └──────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────┐   │
│  │ EasyOCR (Text Extraction)                        │   │
│  └──────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────┐   │
│  │ OpenCV (Video Frame Extraction)                  │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Why this architecture?

| Decision | Reason |
| ---------- | -------- |
| **Rust + Tauri** (not Electron) | Native webview, binary is ~10x smaller, memory footprint is a fraction of Electron |
| **Python worker** (not all-Rust) | CLIP and EasyOCR have mature Python ecosystems; no need to reinvent the wheel |
| **ZeroMQ + Protobuf** (not HTTP) | Microsecond latency, type-safe schema, works fully offline |
| **FlatVector matrix** (not Faiss) | For N < 50,000, O(N·d) brute-force is fast enough (~40ms) and has zero dependencies |
| **SQLite + WAL mode** | Battle-tested, embedded, supports concurrent reads during writes |

### Data Flow

**Indexing a folder**: Rust walks the selected directory, collects file paths, and sends them in batches to the Python worker via ZMQ. Python extracts CLIP embeddings and OCR text for each image or video frame, then returns the results. Rust stores everything in SQLite and the in-memory FlatVector matrix, while the frontend shows real-time progress.

**Searching by text**: The frontend sends a query to Rust, which forwards it to Python for CLIP text embedding. Rust then searches the FlatVector matrix using dot-product similarity, combines the results with OCR, note, and filename scores, and returns ranked results to the frontend.

**Searching by image**: The workflow is the same as text search, except Python generates a CLIP image embedding instead of a text embedding from the reference image.

---

## 🔬 Key Technical Highlights

### Hybrid Scoring Model

FrameScout combines **four signals** into a unified relevance score:

```text
final_score = vector_similarity × 1.0      (semantic meaning)
            + ocr_match         × 2.0      (text in image)
            + note_match        × 2.5      (user annotations)
            + filename_match    × 3.0      (path/name match)
```

Each component can be toggled independently in the UI. Results are scored, ranked, and paginated in real time.

### FlatVector Matrix Search

All 512-dimensional CLIP embeddings are stored in a single `Vec<f32>` for cache-friendly sequential access:

```rust
pub struct FlatVectorMatrix {
    pub dim: usize,            // 512
    pub flat_vectors: Vec<f32>, // [v0_0..v0_511, v1_0..v1_511, ...]
    pub metadata: Vec<ImageMeta>,
}
```

Search is a simple dot-product loop over contiguous memory — **no malloc, no indirection, no external library**. At 50,000 images, a full scan completes in ~40ms.

---

## 📊 Performance

Tested on a desktop with AMD Ryzen 7 5800X + 32GB RAM + NVIDIA RTX 3070.

| Collection Size | Text Query Latency | Image Query Latency | Index Throughput |
| --------------- | ------------------ | ------------------- | -----------------|
| 1,000 images    | 0.8 ms             | 1.1 ms              | ~1,200 img/min   |
| 10,000 images   | 7.2 ms             | 9.5 ms              | ~800 img/min     |
| 50,000 images   | 38.0 ms            | 45.0 ms             | ~600 img/min     |

> **Disclaimer**: These are benchmark results from our development environment (AMD Ryzen 7 5800X, 32GB RAM, RTX 3070). Actual performance depends on your hardware and dataset characteristics. Reproduce on your machine using the provided benchmark script (coming soon).

---

## 🚀 Quick Start (Development)

### Prerequisites

- Rust 1.75+ (2021 edition)
- Python 3.10+
- Node.js 18+ & npm
- CLIP model files (see below)

### Build & Run

```bash
# Clone the repository
git clone https://github.com/bobgsning/FrameScout.git
cd FrameScout

# 1. Install frontend dependencies
cd src/frontend
npm install

# 2. Set up the Python inference worker
cd ../inference-worker
python -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate
pip install -r requirements.txt

# > ⚠️ This step requires internet access. After completion, FrameScout works fully offline.
# 3. Download CLIP model (one-time, ~600MB)
# Option A: Use the provided script
python ../scripts/download_models.py
# Option B: Manually download from Hugging Face and place in:
#   src/inference-worker/models/openai-clip/

# 4. Build and run (Community Edition)
cd ../frontend
npm run tauri dev          # Development mode
# npm run tauri build      # Production build

```

### First Launch

1. The splash screen appears: *"FRAME SCOUT NEURAL LINK ESTABLISHING..."*
2. CLIP + EasyOCR models load into memory (varies depending on hardware)
3. You'll see the main interface with the search console
4. Click **Browse** → select a folder containing images
5. Click **Start Indexing** → watch the real-time extraction bus
6. Type a description in the search bar → get millisecond results

---

## 🔐 Privacy Guarantee

FrameScout was built from the ground up to respect your privacy:

- ✅ **Zero network requests** during normal operation *(One-time model download required on first launch; fully offline afterward)*
- ✅ **Zero telemetry** — no analytics, no usage tracking
- ✅ **Zero cloud dependency** — works in airplane mode
- ✅ **Local-only AI** — all inference runs on your CPU/GPU
- ✅ **Transparent** — open source core, auditable code

You can verify this yourself: run FrameScout with your firewall blocking all outbound traffic. It works perfectly.

---

## 🛠️ Known Limitations (v3.0.0)

We believe in transparency. Here's what FrameScout *doesn't* do yet:

| Limitation | Workaround / Future Plan |
| ---------- | ------------------------ |
| Windows 10/11 only (no macOS/Linux yet) | Cross-platform builds planned for future |
| Video frame extraction uses fixed 1 FPS | Scene-detection based extraction planned |
| FlatVector search is O(N·d) | Will migrate to Faiss/Annoy when N > 50K |
| No filesystem real-time monitoring | Manual re-scan; inotify/watchdog planned |
| No LLM-generated captions | Deliberate — would require network access |

---

## 🗺️ Roadmap

### v3.0.x (Current — Stability & Polish)

- [x] Core search (text + image + OCR + notes)
- [x] Visual clustering
- [x] Smart folders
- [ ] RwLock optimization (fix search-during-scan blocking)
- [ ] Hover tooltips with file metadata
- [ ] Per-image delete (not just bulk ghost purge)
- [ ] Video timeout reduced to 10s + loading animation
- [ ] Score normalization moved to backend

### v3.1 (Cross-Platform & UX)

- [ ] Folder tree sidebar view
- [ ] Multiple view modes (grid / list / timeline)
- [ ] Export/Import database (SQLite backup)
- [ ] Data timeline (visualize your indexing history)

### v3.2 (Performance & Intelligence)

- [ ] Faiss integration for N > 50K
- [ ] Scene-detection for video keyframes
- [ ] GPU acceleration via ONNX Runtime
- [ ] Batch operations (multi-select delete/export)
- [ ] Multi-modal fusion search (text + image simultaneously)
- [ ] macOS build (Apple Silicon native)
- [ ] Linux build (AppImage + Flatpak)

---

## 🤝 Contributing

FrameScout is open source under Apache 2.0. The core architecture, algorithms, and communication protocols are fully available for study and modification.

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### What's Open Source

| Component | License | Notes |
| --------- | ------- | ----- |
| Rust/Tauri core (`src/frontend/src-tauri/`) | Apache 2.0 | Full source available |
| Vue 3 frontend (`src/frontend/`) | Apache 2.0 | Full source available |
| Protobuf definitions (`src/proto/`) | Apache 2.0 | Full source available |
| Python inference worker | Apache 2.0 | Included in this repository |

---

## 📄 License

FrameScout core is licensed under **Apache License 2.0**. See [LICENSE](LICENSE) for details.

---

## ™️ Trademarks

FrameScout and its logo are trademarks of **AetherFlow Labs Inc.**

All other trademarks are the property of their respective owners.

---

## 🙏 Acknowledgments

- **OpenAI CLIP** — for the open-source contrastive language-image pre-training model
- **EasyOCR** — for the lightweight, multi-language OCR engine
- **Tauri** — for the secure, lightweight desktop app framework
- **ZeroMQ** — for reliable, high-performance inter-process communication
- **Prost** — for idiomatic Protobuf support in Rust
- **The Rust Community** — for building a language where safety and performance coexist

---

## 📞 Contact & Support

- **Issues & Bug Reports**: [GitHub Issues](https://github.com/bobgsning/FrameScout/issues)
- **General Questions**: <bobgsning@outlook.com>
- **Security Concerns**: Please email directly (PGP key available on request)

---

**FrameScout — Your images. Your privacy. Your search.**

🔒 100% Offline · ⚡ Millisecond Search · 🖥️ Local AI Inference
