# 🔍 FrameScout — Offline AI Search

## *100% Private, Fully Offline, Multi-Modal Desktop Search Engine*

**Search your local images and videos with natural language, OCR text, or visual similarity — no cloud, no telemetry, no accounts. Free & open source (Community Edition).**

[![License: Apache 2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4)](https://www.microsoft.com/windows)
![Architecture](https://img.shields.io/badge/architecture-Rust%20%2B%20Python%20%2B%20Vue-ff69b4)
[![AI-Assisted](https://img.shields.io/badge/AI--assisted-development-brightgreen)](AI_POLICY.md)

FrameScout is a **fully offline, AI-powered visual search engine** for your local images and videos.  
Type *"sunset beach with friends"*, paste a reference image, or search by text inside images — and get results in milliseconds. Everything runs on your machine. Nothing leaves your computer.

⬇️ [Download the latest release](https://github.com/bobgsning/FrameScout/releases) ·
📖 [Quick Start](#-quick-start-development) ·
🤝 [Contributing](CONTRIBUTING.md)

> 🆕 **v3.0.2 is here!** New files now appear first thanks to index-time sorting. Try the new **“Show All”** button for a complete overview. [See changelog](CHANGELOG.md#302---2026-08-09).

> **Just want to try it?** Download the latest Windows executable from the [https://github.com/bobgsning/FrameScout/releases](https://github.com/bobgsning/FrameScout/releases) — no installation required. Unzip and run.

---

## 🎯 The Problem

You have hundreds of gigabytes of screenshots, photos, and video clips scattered across your drives.  
Existing solutions force you to choose between two bad options:

☁️ **Upload everything to the cloud** — and lose your privacy forever.  
📁 **Stick with local file explorers** — and spend hours hunting through folders.

FrameScout offers a **third way**: AI-powered search that runs entirely on your machine. No uploads. No subscriptions. No compromises.

---

## 🛠️ Known Limitations (v3.0.2)

We believe in transparency. Here's what FrameScout *doesn’t* do yet:

| Limitation | Workaround / Future Plan |
| ---------- | ------------------------ |
| Windows 10/11 only (no macOS/Linux yet) | Cross-platform builds planned for future |
| Video frame extraction uses fixed 1 FPS | Scene-detection based extraction planned |
| FlatVector search is O(N·d) | Will migrate to Faiss/Annoy when N > 50K |
| No filesystem real-time monitoring | Manual re-scan; inotify/watchdog planned |
| No LLM-generated captions | Deliberate — would require network access |
| “Show All” mode may lag with >10K files | Virtual scrolling planned for v3.1 |

---

## ✨ Features

| Feature | Description |
| --------- | ------------- |
| 🔒 **100% Offline** | No internet required. Models are bundled locally. Your data never leaves your computer. |
| 💡 **Semantic Search (ONNX + SigLIP 2)** | Type words and find matching images by meaning, not just filenames. |
| 🖼️ **Image-to-Image Search** | Drop a reference image to find visually similar ones in your library. |
| 🔍 **OCR Text Search** | Extracts and indexes text from images. Like searching *"receipt from March"* and find it instantly. |
| 📋 **Show All Mode** | Load every indexed file. New files are sorted by intake time and always appear at the top. |
| 🎬 **Video Frame Indexing** | Automatically extracts key frames from videos and indexes them alongside static images. |
| 🧩 **Visual Clustering** | Discover groups of similar images (duplicates, near-duplicates, burst shots) with one click. |
| 📁 **Smart Folders** | Save any search as a dynamic folder that updates automatically when new files are added. |
| 📝 **Personal Notes** | Attach markdown notes to any image. Notes are searchable. Next time you’ll find it more effortlessly. |

### 🎬 Video Frame Semantic Search with Instant Seek

FrameScout doesn't just find videos — it pinpoints the **exact timestamp**:

- **Semantic Matching**: Search using natural language (e.g., *"sunset under the pier"* or *"code snippet on screen"*).
- **One-Click Instant Seek**: Click any search result to jump directly to that exact second (`01:23:45`) in your video.

> **New in v3.0.2**: Newly indexed files are sorted by their ingestion time, so the most recent additions always appear first in browse mode. Click the “📋 Show All” button to see every file at once.

---

## 📁 Repository Structure (Key Directories)

```text
FrameScout/
├── src/
│   ├── frontend/               # Vue 3 + Tauri desktop app (Rust core)
│   ├── inference-worker/       # Python AI inference engine (ONNX + SigLIP 2 + EasyOCR)
│   │   └── models/
│   │       ├── siglip2-base/   # SigLIP 2 ONNX models (vision + text)
│   │       └── easyocr/        # EasyOCR model storage (offline)
│   ├── proto/                  # ZeroMQ communication schema (Protobuf)
├── scripts/                    # Utility scripts (model download, etc.)
├── README.md
├── CONTRIBUTING.md
└── LICENSE
```

Full directory tree available in [TREE.md](./TREE.md).

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
│  │ WalkDir  │  │ Rusqlite     │  │ Matrix (768D)  │     │
│  └──────────┘  └──────────────┘  └────────────────┘     │
└──────────────────────┬──────────────────────────────────┘
                       │  ZeroMQ + Protobuf (REQ/REP)
                       ▼
┌─────────────────────────────────────────────────────────┐
│                Python Inference Worker                  │
│  ┌──────────────────────────────────────────────────┐   │
│  │ ONNX + SigLIP 2  (Text + Image Embeddings, 768D) │   │
│  └──────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────┐   │
│  │ EasyOCR (Text Extraction, offline)               │   │
│  └──────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────┐   │
│  │ OpenCV (Video Frame Extraction, 1 FPS)           │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Why this architecture?

| Decision | Reason |
| -------- | ------ |
| **Rust + Tauri** (not Electron) | Native webview, binary is ~10x smaller, memory footprint is a fraction of Electron |
| **Python worker** (not all-Rust) | SigLIP 2 and EasyOCR have mature Python ecosystems; no need to reinvent the wheel |
| **ONNX Runtime** (not PyTorch) | Universal GPU acceleration (NVIDIA CUDA, AMD DirectML, Intel, CPU fallback); smaller footprint |
| **ZeroMQ + Protobuf** (not HTTP) | Microsecond latency, type-safe schema, works fully offline |
| **FlatVector matrix** (not Faiss) | For N < 50,000, O(N·d) brute-force is fast enough (~40ms) and has zero dependencies |
| **SQLite + WAL mode** | Battle-tested, embedded, supports concurrent reads during writes |

### Data Flow

**Indexing a folder**: Rust walks the selected directory, collects file paths, and sends them in batches to the Python worker via ZMQ. Python generates SigLIP 2 image embeddings (768D) and OCR text for each image or video frame, then returns the results. Rust stores everything in SQLite and the in-memory FlatVector matrix, while the frontend shows real-time progress.

**Searching by text**: The frontend sends a query to Rust, which forwards it to Python for SigLIP 2 text embedding. Rust then searches the FlatVector matrix using dot-product similarity, combines the results with OCR, note, and filename scores, and returns ranked results to the frontend.

**Searching by image**: The workflow is the same as text search, except Python generates a SigLIP 2 image embedding instead of a text embedding from the reference image.

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

All 768‑dimensional SigLIP 2 embeddings are stored in a single `Vec<f32>` for cache-friendly sequential access:

```rust
pub struct FlatVectorMatrix {
    pub dim: usize,            // 768
    pub flat_vectors: Vec<f32>, // [v0_0..v0_767, v1_0..v1_767, ...]
    pub metadata: Vec<ImageMeta>,
}
```

Search is a simple dot-product loop over contiguous memory — **no malloc, no indirection, no external library**. At 50,000 images, a full scan completes in ~40ms.

---

## 📊 Performance

Tested on a desktop with AMD Ryzen 7 5800X + 32GB RAM + NVIDIA RTX 3070 (ONNX with CUDA).

| Collection Size | Text Query Latency | Image Query Latency | Index Throughput |
| --------------- | ------------------ | ------------------- | -----------------|
| 1,000 images    | 0.8 ms             | 1.1 ms              | ~1,200 img/min   |
| 10,000 images   | 7.2 ms             | 9.5 ms              | ~800 img/min     |
| 50,000 images   | 38.0 ms            | 45.0 ms             | ~600 img/min     |

> **Disclaimer**: These are benchmark results from our development environment. Actual performance depends on your hardware and dataset characteristics. Reproduce on your machine using the provided benchmark script (coming soon). v3.0.2 introduces index-time ordering, which does not affect query latency.

---

## 🚀 Quick Start (Development)

### Prerequisites

- Rust 1.75+ (2021 edition)
- Python 3.10+
- Node.js 18+ & npm
- SigLIP 2 ONNX model files (see below)

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
# 3. Download SigLIP 2 + EasyOCR models (one-time, ~1.5GB total) (from src/inference-worker)
python ../../scripts/download_models.py

# 4. Build and run (Community Edition)
# Return to frontend directory for building
cd ../frontend
npm run tauri dev          # Development mode
# npm run tauri build      # Production build
```

### Packaging the Inference Worker

The Python inference worker must be compiled into a standalone executable for Tauri to launch it.

```bash
# From the repository root
cd src/inference-worker
# Activate your virtual environment first
pyinstaller --noconfirm --onedir --console --name "ai_worker" \
    --hidden-import "transformers" \
    --hidden-import "easyocr" \
    main.py
```

After successful build, copy the output to the Tauri binary folder:

```bash
cp -r dist/ai_worker ../frontend/src-tauri/bin/
```

Now `npm run tauri dev` will automatically start the compiled worker.

> **Note:** The packaged worker looks for models in `./models/` relative to its own location. After copying to `src/frontend/src-tauri/bin/`, ensure the `models` folder exists there, or adjust the path in `main.py`.

### First Launch

1. The splash screen appears: *"FRAME SCOUT NEURAL LINK ESTABLISHING..."*
2. SigLIP 2 + EasyOCR models load into memory (varies depending on hardware)
3. You'll see the main interface with the search console
4. Click **Browse** → select a folder containing images
5. Click **Start Indexing** → watch the real-time extraction bus
6. Type a description in the search bar → get millisecond results

---

## 🗺️ Roadmap

### v3.0.x (Current — Stability & Polish)

- [x] Core search (text + image + OCR + notes)
- [x] Visual clustering
- [x] Smart folders
- [x] SigLIP 2 migration (768D embeddings, ONNX Runtime)
- [x] Index-time sorting (new files always appear first)
- [x] “Show All” mode (flat listing without pagination)
- [x] RwLock optimization (non-blocking search during scan)
- [ ] Hover tooltips with file metadata
- [ ] Per-image delete (not just bulk ghost purge)
- [ ] Video timeout reduced to 10s + loading animation

### v3.1 (Cross-Platform & UX)

- [ ] Folder tree sidebar view
- [ ] Enhance smart folders and other user experiences
- [ ] Multiple view modes (grid / list / timeline)
- [ ] Export/Import database (SQLite backup)
- [ ] Data timeline (visualize your indexing history)

### v3.2 (Performance & Intelligence)

- [ ] Faiss integration for N > 50K
- [ ] Scene-detection for video keyframes
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

"FrameScout", "FrameScout — Offline AI Search", and the FrameScout logo are trademarks of Bob G. S. Ning.

All other trademarks are the property of their respective owners.

---

## 🙏 Acknowledgments

- **Google SigLIP 2** — for the open-source contrastive language-image pre-training model (used via ONNX)
- **EasyOCR** — for the lightweight, multi-language OCR engine
- **ONNX Runtime** — for cross-platform, hardware-accelerated inference
- **Tauri** — for the secure, lightweight desktop app framework
- **ZeroMQ** — for reliable, high-performance inter-process communication
- **Prost** — for idiomatic Protobuf support in Rust
- **The Rust Community** — for building a language where safety and performance coexist

---

## 📞 Contact & Support

- **Issues & Bug Reports**: [GitHub Issues](https://github.com/bobgsning/FrameScout/issues)
- **General Questions**: <bobgsning@outlook.com>
- **Security Concerns**: Please email directly (PGP key available on request)

> 🤝 **We need your help!** FrameScout is a one-person project right now. If you're passionate about privacy-first AI tools, we'd love your contribution — whether it's code, documentation, bug reports, or just testing on your machine.
> We're currently preparing our first good first issues. In the meantime, feel free to [open a discussion](https://github.com/bobgsning/FrameScout/discussions) or [browse the codebase](https://github.com/bobgsning/FrameScout).

---

**FrameScout — Your images. Your privacy. Your search.**

🔒 100% Offline · ⚡ Millisecond Search · 🖥️ Local AI Inference
