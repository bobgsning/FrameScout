# 🔍 FrameScout — Offline AI Search

<p align="center">
  <a href="./README.md"><strong>English</strong></a> ·
  <a href="./README.zh-CN.md">中文</a>
</p>

## *100% Private, Fully Offline, Multi-Modal Desktop Search Engine*

**Search your local images and videos with natural language, OCR text, or visual similarity — no cloud, no telemetry, no accounts. Free & open source (Global Edition).**

[![License: Apache 2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4)](https://www.microsoft.com/windows)
![Architecture](https://img.shields.io/badge/architecture-Rust%20%2B%20Python%20%2B%20Vue-ff69b4)
[![AI-Assisted](https://img.shields.io/badge/AI--assisted-development-brightgreen)](AI_POLICY.md)

FrameScout is a **fully offline, AI-powered visual search engine** for your local images and videos.  
Type *"sunset beach with friends"*, paste a reference image, or search by text inside images — and get results in milliseconds. Everything runs on your machine. Nothing leaves your computer.

⬇️ [Download the latest release](https://github.com/bobgsning/FrameScout/releases) ·
📖 [Quick Start](#-quick-start-development) ·
🤝 [Contributing](CONTRIBUTING.md)

> 🆕 **v3.2.0 is here!** A sweeping polish pass across the whole product: normalized hybrid scoring (semantic search no longer drowned out by filename matches), scene-detection video frame extraction, FTS5 fault-tolerant text search, lightbox preview with OCR hit red-boxes, right-click open/reveal/copy, drag-to-search, scan pause/cancel with ETA, multi-select + CSV/Markdown/JSON export, database backup, and a full data-consistency pass. [See changelog](CHANGELOG.md#320---2026-10-08).
>
> 🌐 **Bilingual interface**: full English & Simplified Chinese localization, English by default. Switch anytime in **Settings → Language**.

---

> **Just want to try it?** Download the latest Windows executable from the [https://github.com/bobgsning/FrameScout/releases](https://github.com/bobgsning/FrameScout/releases) — no installation required. Unzip and run.

---

## 🎯 The Problem

You have hundreds of gigabytes of screenshots, photos, and video clips scattered across your drives.  
Existing solutions force you to choose between two bad options:

☁️ **Upload everything to the cloud** — and lose your privacy forever.  
📁 **Stick with local file explorers** — and spend hours hunting through folders.

FrameScout offers a **third way**: AI-powered search that runs entirely on your machine. No uploads. No subscriptions. No compromises.

---

## 🛠️ Known Limitations

We believe in transparency. Here's what FrameScout *doesn't* do yet — and what we plan to do about it:

| Limitation | Workaround / Future Plan |
| ---------- | ------------------------ |
| Windows 10/11 only (no macOS/Linux yet) | Cross-platform builds planned |
| FlatVector search is O(N·d) | Will migrate to Faiss/Annoy when N > 50K |
| No filesystem real-time monitoring | Manual re-scan; inotify/watchdog planned |
| No LLM-generated captions | Deliberate — would require network access |
| OCR supports English & Simplified Chinese only | More languages planned (RapidOCR rec-model swap) |
| OCR on-demand is per-file only | Batch OCR for multiple files planned |
| PDF/DOCX ingestion not yet supported | Convert to .txt/.md first; text extraction planned |
| Database backup is one-way (no in-app restore) | Restore by copying the .db back; in-app restore planned |
| Smart folders save a single text query + channel toggles (no boolean/time/type/seed conditions yet) | Full condition editor planned |
| FTS5 full-text search covers imported text entries only (not OCR / filenames / notes) | Expanding coverage planned |
| No partition / batch / date-range search filter | Planned |
| Relevance thresholds & video frame-extraction limits are fixed, not user-adjustable | Settings planned |
| Score breakdown bars are approximate for literal channels | Exact per-channel scores planned |

---

## ✨ Features

| Feature | Description |
| --------- | ------------- |
| 🔒 **100% Offline** | No internet required. Models are bundled locally. Your data never leaves your computer. |
| 💡 **Semantic Search (ONNX + SigLIP 2)** | Type words and find matching images by meaning, not just filenames. |
| 🖼️ **Image-to-Image Search** | Drop a reference image to find visually similar ones in your library. |
| 🔍 **OCR Text Search** | Extracts and indexes text from images (English & Simplified Chinese). Search *"receipt from March"* and find it instantly. Supports on-demand OCR on selected files. |
| 📋 **Show All Mode** | Load every indexed file. New files are sorted by intake time and always appear at the top. |
| 🎬 **Video Frame Indexing** | Automatically extracts key frames from videos (scene detection) and indexes them alongside static images. |
| 🧩 **Visual Clustering** | Discover groups of similar images (duplicates, near-duplicates, burst shots) with one click. |
| 📁 **Smart Folders** | Save a text search as a named, persistent folder that re-runs against your library whenever you open it. |
| 📝 **Personal Notes** | Attach markdown notes to any image. Notes are searchable. Next time you'll find it more effortlessly. |

### 🎬 Video Frame Semantic Search with Instant Seek

FrameScout doesn't just find videos — it pinpoints the **exact timestamp**:

- **Semantic Matching**: Search using natural language (e.g., *"sunset under the pier"* or *"code snippet on screen"*).
- **One-Click Instant Seek**: Click any search result to jump directly to that exact second (`01:23:45`) in your video.

---

## 📁 Repository Structure (Key Directories)

```text
FrameScout/
├── src/
│   ├── FrameScout-UI/               # Vue 3 + Tauri desktop app (Rust core)
│   ├── inference-worker/            # Python AI inference engine (ONNX + SigLIP 2 + RapidOCR)
│   │   └── models/                  # Downloaded by scripts/download_models.py (git-ignored)
│   │       ├── siglip2-base/        # SigLIP 2 ONNX models (vision + text)
│   │       └── bge-m3/              # BGE-M3 text embeddings (optional)
│   ├── proto/                       # ZeroMQ communication schema (Protobuf)
├── scripts/                         # Utility scripts (model download, etc.)
├── README.md
├── CONTRIBUTING.md
└── LICENSE
```

> OCR (RapidOCR / PP-OCRv4) weights ship inside the `rapidocr_onnxruntime` wheel — nothing extra to download for English & Simplified Chinese.

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
│  │ RapidOCR (Text Extraction, offline)              │   │
│  └──────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────┐   │
│  │ OpenCV (Video Frame Extraction, scene-detection) │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Why this architecture?

| Decision | Reason |
| -------- | ------ |
| **Rust + Tauri** (not Electron) | Native webview, binary is ~10x smaller, memory footprint is a fraction of Electron |
| **Python worker** (not all-Rust) | SigLIP 2 and RapidOCR have mature Python ecosystems; no need to reinvent the wheel |
| **ONNX Runtime** (not PyTorch) | Universal GPU acceleration (NVIDIA CUDA, AMD DirectML, Intel, CPU fallback); smaller footprint |
| **ZeroMQ + Protobuf** (not HTTP) | Microsecond latency, type-safe schema, works fully offline |
| **FlatVector matrix** (not Faiss) | For N < 50,000, O(N·d) brute-force is fast enough (~40ms) and has zero dependencies |
| **SQLite + WAL mode** | Battle-tested, embedded, supports concurrent reads during writes |

### Data Flow

**Indexing a folder**: Rust walks the selected directory, collects file paths, and sends them in batches to the Python worker via ZMQ. Python generates SigLIP 2 image embeddings (768D) and OCR text for each image or video frame, then returns the results. Rust stores everything in SQLite and the in-memory FlatVector matrix, while the frontend shows real-time progress.

**Searching by text**: The frontend sends a query to Rust, which forwards it to Python for SigLIP 2 text embedding. Rust searches the FlatVector matrix by dot-product similarity and returns raw per-channel scores (semantic, OCR, note, filename) to the frontend, which normalizes and fuses them into a final ranked result.

**Searching by image**: The same flow as text search, except Python generates a SigLIP 2 image embedding from the reference image instead of a text embedding.

---

## 🔬 Key Technical Highlights

### Hybrid Scoring Model

FrameScout combines **four signals** into a unified relevance score. Since v3.2.0 the backend returns **raw, independently computed signals** — the SigLIP cosine similarity and per-channel literal hit flags — and the frontend normalizes each one before fusing, so semantic matches are no longer drowned out by filename matches:

```text
fused_score = sigmoid(cosine)   × semantic_weight  (semantic meaning)
            + ocr_hit           × ocr_weight        (text in image)
            + note_hit          × note_weight       (user annotations)
            + filename_hit      × filename_weight   (path/name match)
```

The normalization and fusion live in the frontend (`utils/score.ts`), and each channel can be toggled independently in the UI. Results are scored, ranked (with a stable tie-breaker), and paginated in real time. *Note: the fusion weights and relevance thresholds are currently fixed constants — see Known Limitations.*

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

> **Disclaimer**: These are benchmark results from our development environment. Actual performance depends on your hardware and dataset characteristics. Reproduce on your machine with `python src/inference-worker/scripts/benchmark.py --library <path>` (writes `benchmarks/YYYY-MM-DD.json`).

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
cd src/FrameScout-UI
npm install

# 2. Set up the Python inference worker
cd ../inference-worker
python -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate
pip install -r requirements.txt

# > ⚠️ This step requires internet access. After completion, FrameScout works fully offline.
# 3. Download SigLIP 2 + BGE-M3 models (one-time, ~1.5GB total) (from src/inference-worker)
python ../../scripts/download_models.py

# 4. Build and run (Global Edition)
# Return to frontend directory for building
cd ../FrameScout-UI
npm run tauri dev          # Development mode
# npm run tauri build      # Production build
```

### Packaging the Inference Worker

The Python inference worker must be compiled into a standalone executable for Tauri to launch it.
The preferred packager is **Nuitka** (no temp-extraction delay, ~30–50% smaller, millisecond startup);
PyInstaller is a legacy fallback.

```powershell
# From the repository root
cd src/inference-worker
# Activate your virtual environment first, then:
.\build_nuitka.ps1            # release (standalone + onefile)
# .\build_nuitka.ps1 -Onedir # debug (standalone only, faster to build)
```

The script handles the DirectML `capi` DLLs, the wheel-bundled RapidOCR weights, the external
`models\` resources, and deployment to `..\FrameScout-UI\src-tauri\bin\ai_worker\` automatically.
See `src/inference-worker/BUILD.md` §4 for details.

> **Note:** The packaged worker looks for models in `./models/` relative to its own location.
> The Nuitka script copies `models\` next to the exe for you.

### First Launch

1. The splash screen appears: *"FRAME SCOUT NEURAL LINK ESTABLISHING..."*
2. SigLIP 2 + RapidOCR + BGE-M3 models load into memory (varies depending on hardware)
3. You'll see the main interface with the search console
4. Click **Browse** → select a folder containing images
5. Click **Start Indexing** → watch the real-time extraction bus
6. Type a description in the search bar → get millisecond results

---

## 🗺️ Roadmap

> **A note on honesty**: FrameScout is actively developed by a small team. The items below are implemented and functional, but a few are still in a "first version" state — the detailed gaps are listed up front in [Known Limitations](#-known-limitations). Everything marked "planned" is genuinely not built yet.

### v3.2.0 (Current — 2026-10-08)

- [x] Core search (text + image + OCR + notes)
- [x] SigLIP 2 migration (768D embeddings, ONNX Runtime)
- [x] Normalized hybrid scoring (semantic no longer drowned by filename matches)
- [x] Visual clustering (medoid representative, sampled for large libraries)
- [x] Smart folders (persistent, renamable, real counts, delete confirmation) — *single-query scope*
- [x] "Show All" mode (flat listing without pagination)
- [x] Index-time sorting (new files always appear first)
- [x] RwLock optimization (non-blocking search during scan)
- [x] Query preprocessing (trim + NFKC + tokenization + negative terms)
- [x] Scene-detection video frame extraction + adjacent-frame dedup
- [x] FTS5 trigram fault-tolerant text search — *text entries only*
- [x] Lightbox preview (zoom / rotate / fullscreen / OCR hit red-boxes)
- [x] Right-click open / reveal-in-explorer / copy path
- [x] Drag-and-drop image-to-search
- [x] Scan pause/cancel + ETA + failed-file retry
- [x] Multi-select + candidate set + CSV/Markdown/JSON export
- [x] Global Toast notifications + keyboard shortcuts (Esc / `/` / arrows / Space)
- [x] Search history + de-jargoned search modes
- [x] Frame-accurate video notes + Markdown note rendering
- [x] Database backup / integrity check — *one-way (no in-app restore)*
- [x] Data timeline + "on this day last year"
- [x] RRF cross-channel fusion (`search_unified`)
- [x] RPC timeout convergence (no more infinite hangs)
- [x] Unified search wired to the frontend + per-channel score-source bars
- [x] Settings page + preferences persistence (path / page-size remembered across restarts)
- [x] Scan stop button + cancelled batches distinguished in reports
- [x] Disk-offline ghost protection (offline drives greyed out & purge blocked)
- [x] Batch note editing (apply one note to many selected files)

### v3.3 (Planned)

- [ ] Faiss integration for N > 50K
- [ ] Smart-folder condition editor (boolean / time / file-type / seed-image conditions)
- [ ] FTS5 coverage for OCR / filenames / notes
- [ ] Partition / batch / date-range search
- [ ] In-app database restore
- [ ] Multi-modal fusion search (text + image simultaneously)
- [ ] Binary PDF/DOCX text extraction
- [ ] User-adjustable relevance thresholds & video extraction limits
- [ ] macOS build (Apple Silicon native)
- [ ] Linux build (AppImage + Flatpak)
- [ ] Folder tree sidebar view
- [ ] Per-image delete (not just bulk ghost purge)
- [ ] Virtual scrolling for very large libraries

---

## 🤝 Contributing

FrameScout is open source under Apache 2.0. The core architecture, algorithms, and communication protocols are fully available for study and modification.

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### What's Open Source

| Component | License | Notes |
| --------- | ------- | ----- |
| Rust/Tauri core (`src/FrameScout-UI/src-tauri/`) | Apache 2.0 | Full source available |
| Vue 3 frontend (`src/FrameScout-UI/`) | Apache 2.0 | Full source available |
| Protobuf definitions (`src/proto/`) | Apache 2.0 | Full source available |
| Python inference worker | Apache 2.0 | Included in this repository |

---

## 📄 License

FrameScout core is licensed under **Apache License 2.0**. See [LICENSE](LICENSE) for details.

© 2026 AetherFlow Labs Inc.

---

## ™️ Trademarks

"FrameScout", "FrameScout — Offline AI Search", and the FrameScout logo are trademarks of **Bob G. S. Ning**.

All other trademarks are the property of their respective owners.

---

## 🙏 Acknowledgments

- **Google SigLIP 2** — for the open-source contrastive language-image pre-training model (used via ONNX)
- **RapidOCR** — for the lightweight, multi-language OCR engine
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

> 🤝 **We need your help!** FrameScout is a small project right now. If you're passionate about privacy-first AI tools, we'd love your contribution — whether it's code, documentation, bug reports, or just testing on your machine.
> We're currently preparing our first good first issues. In the meantime, feel free to [open a discussion](https://github.com/bobgsning/FrameScout/discussions) or [browse the codebase](https://github.com/bobgsning/FrameScout).

---

**FrameScout — Your images. Your privacy. Your search.**

🔒 100% Offline · ⚡ Millisecond Search · 🖥️ Local AI Inference
