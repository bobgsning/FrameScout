# 🔍 FrameScout — Offline AI Search

<p align="center">
  <img src="src/FrameScout-UI/app.svg" alt="FrameScout Logo" width="150" />
</p>

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

> 🆕 **v3.3.0 is here!** The biggest release yet — a full logic-search refactor plus a data-safety pass:
> - 🧮 **Condition-based search** — build a query as visible rows (keyword / date / batch / file type / seed image) combined with AND / OR / exclude and per-condition thresholds. What you type is taken literally — there's no symbol syntax to learn; `e-mail`, `-5°C`, and `会议@2024` are matched exactly as written. Smart folders now save the complete condition set and run the exact same evaluator as the search bar.
> - 💾 **Backup & restore** — one-click timestamped backups with rotation and integrity check, plus in-app restore: validity-checked preview with a frame-count comparison, and an automatic backup taken before restoring.
> - 🗂️ **Partitioned search** — filter results by source batch, root-path prefix, or index-time range (also available on image search).
> - 🧹 **Automatic cleanup policy** — event retention & backup rotation with toggles and a startup report, so nothing is ever deleted silently.
> - 🌐 **True i18n** — the backend now returns stable error codes instead of English sentences, rendered in all 9 UI languages; zh-CN says 「文字识别」, zh-TW 「文字辨識」.
> - 🎞️ **FFmpeg fallback + clip cutting** — MKV/AVI containers that OpenCV can't decode fall back to a bundled, hash-verified LGPL FFmpeg; cut a short clip around any hit frame straight from a card or the lightbox.
>
> [See changelog](CHANGELOG.md#330---2026-10-18) · [Release notes](https://github.com/bobgsning/FrameScout/releases)
>
> 🌐 **Multilingual interface**: 9 UI languages — English, 简体中文, 繁體中文, 日本語, Русский, 한국어, Deutsch, Français, Español — English by default. Switch anytime in **Settings → Language**.
>
> 🔤 **Multi-language OCR**: recognition for 4 built-in languages (English, 简体中文, 繁體中文, 日本語) plus 6 on-demand downloadable recognition models (Русский, 한국어, Deutsch, Français, Español, and an English-optimized variant), with more coming soon. See [Localization & OCR Languages](#-localization--ocr-languages).

---

> **Just want to try it?** Download the latest Windows executable from the [releases page](https://github.com/bobgsning/FrameScout/releases) — no installation required. Unzip and run. The prebuilt app includes a free trial of **100 indexed frames** (videos count per extracted frame); the source is fully open under Apache 2.0 — the trial limit lives in the source too, so you're free to modify or remove it and [build your own version](#-quick-start-development).

---

## 🧮 Logic Search — query your archive the way you think

Most "AI search" makes you type one sentence and hope. FrameScout lets you **compose a query from visible condition rows** and join them with **AND / OR / exclude**, each row with its own **threshold**:

- **Keyword** — match by meaning, OCR text, note, or filename
- **Date** — e.g. *"only things from 2022"*
- **Batch** — the import/source a file came from
- **Type** — image or video
- **Seed image** — find visuals similar to a reference picture

Looking for *"the Tokyo trip with the red sign, but no people"*? That's three rows, not a wish. And because there's **no symbol syntax to learn**, everything you type is matched literally — `e-mail`, `-5°C`, and `会议@2024` are searched exactly as written. Saved searches (Smart Folders) store the very same condition set and run through the same evaluator as the search bar, so a folder is just a query you don't have to retype.

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
| OCR covers 4 built-in + 6 downloadable recognition models (5 more languages + 1 English-optimized variant); 6 more planned | More RapidOCR rec-model weights from the official model library |
| On-demand OCR runs from a card or the multi-select candidate set; no whole-library OCR button yet | Whole-library OCR entry planned |
| PDF/DOCX ingestion not yet supported | Convert to .txt/.md first; text extraction planned |
| In-app database restore takes effect on the next launch (pending-restore marker), not instantly | Seamless in-session restore planned |
| Smart folders support full condition rows (AND/OR/negative/threshold/date/batch/seed image), but the backend can't pre-count condition-row folders (shown as "?") | Server-side counting for condition folders planned |
| FTS5 full-text search covers imported text entries only (not OCR / filenames / notes) | Expanding coverage planned |
| Video frame-extraction limits (fps / max frames / max duration) are fixed constants | Adjustable settings planned |
| Hybrid fusion weights are fixed constants (relevance sensitivity is adjustable in three steps) | Per-channel weight tuning planned |

---

## ✨ Features

| Feature | Description |
| --------- | ------------- |
| 🔒 **100% Offline** | No internet required. Models are bundled locally. Your data never leaves your computer. |
| 💡 **Semantic Search (ONNX + SigLIP 2)** | Type words and find matching images by meaning, not just filenames. |
| 🖼️ **Image-to-Image Search** | Drop a reference image to find visually similar ones in your library. |
| 🔍 **OCR Text Search** | Extracts and indexes text from images (English, 简体中文, 繁體中文, 日本語 built-in; more downloadable). Search *"receipt from March"* and find it instantly. Supports on-demand OCR on selected files. |
| 📋 **Show All Mode** | Load every indexed file. New files are sorted by intake time and always appear at the top. |
| 🎬 **Video Frame Indexing** | Automatically extracts key frames from videos (scene detection) and indexes them alongside static images. |
| 🧩 **Visual Clustering** | Discover groups of similar images (duplicates, near-duplicates, burst shots) with one click. |
| 🧮 **Condition-Based Search** | Build a query as visible rows — keyword / date / batch / file type / seed image — combined with AND / OR / exclude and per-condition thresholds. No symbol syntax to memorize. |
| 📁 **Smart Folders** | Save a search — a plain query or multi-row conditions (date / batch / seed image, combined with AND / OR / negative terms) — as a named, persistent folder that re-runs against your library whenever you open it. Import / export as JSON. |
| 💾 **Database Backup & Restore** | One-click timestamped backups (rotating) + integrity check + in-app restore with preview and pre-restore auto-backup. |
| 🎞️ **Video Clip Cutting** | Cut a short clip around any hit frame, straight from a result card or the lightbox. |
| 📝 **Personal Notes** | Attach markdown notes to any image. Notes are searchable. Next time you'll find it more effortlessly. |

### 🎬 Video Frame Semantic Search with Instant Seek

FrameScout doesn't just find videos — it pinpoints the **exact timestamp**:

- **Semantic Matching**: Search using natural language (e.g., *"sunset under the pier"* or *"code snippet on screen"*).
- **One-Click Instant Seek**: Click any search result to jump directly to that exact second (`01:23:45`) in your video.
- **Robust Decoding**: containers OpenCV can't open (some MKV/AVI codecs) fall back to a bundled, hash-verified LGPL FFmpeg — no more silent zero-frame indexing.

---

## 🌐 Localization & OCR Languages

FrameScout ships with a **9-language interface** and a **three-tier OCR recognition** system, configured in **Settings**:

**Interface language (9)** — switch instantly in *Settings → Language*: English, 简体中文, 繁體中文, 日本語, Русский, 한국어, Deutsch, Français, Español (English by default).

**OCR recognition languages (three tiers)** — pick any combination in the *OCR recognition languages* list:

| Tier | Languages | Notes |
| ---- | --------- | ----- |
| 🟢 Built-in recognition | English, 简体中文, 繁體中文, 日本語 | Ship with the app; 繁體中文 & 日本語 models can be deleted |
| 🔵 Downloadable recognition | Русский, 한국어, Deutsch, Français, Español + English (optimized) | Models download on demand with progress, resume, SHA-256 verification & disk-space check; removable anytime |
| ⚪ Coming soon | Português, Italiano, Polski, Türkçe, Tiếng Việt, Bahasa Indonesia | Planned — shown in the list but not yet available |

OCR recognition models are obtained from the official [RapidAI/RapidOCR](https://github.com/RapidAI/RapidOCR) model library (verified against official SHA-256 hashes) and stored under `models/ocr/`. Language metadata lives in a single source of truth — `src/inference-worker/ocr_manifest.json`.

## 📁 Repository Structure (Key Directories)

```text
FrameScout/
├── src/
│   ├── FrameScout-UI/               # Vue 3 + Tauri desktop app (Rust core)
│   │   ├── src/                     # Frontend: components / composables / i18n (9 locales) / utils
│   │   └── src-tauri/               # Rust backend: commands / storage / services / model_code
│   ├── inference-worker/            # Python AI inference engine (ONNX + SigLIP 2 + RapidOCR)
│   │   └── models/                  # Downloaded by scripts/download_models.py (git-ignored)
│   │       ├── siglip2-base/        # SigLIP 2 ONNX models (vision + text)
│   │       ├── bge-m3/              # BGE-M3 text embeddings (optional)
│   │       └── ocr/                 # OCR rec models per language (en_opt / ru / ko / de / fr / es on demand)
│   ├── proto/                       # ZeroMQ communication schema (Protobuf)
├── scripts/                         # Utility scripts (model download, i18n check, ...)
├── README.md
├── README.zh-CN.md
├── CONTRIBUTING.md
├── CHANGELOG.md
├── THIRD_PARTY_NOTICES.md           # Third-party components & model weights (incl. bundled FFmpeg)
├── COPYING.LGPLv2.1                 # LGPL v2.1 full text (bundled FFmpeg)
├── AI_POLICY.md
└── LICENSE
```

> OCR (RapidOCR / PP-OCRv4) weights ship inside the `rapidocr_onnxruntime` wheel for English & Simplified Chinese. Traditional Chinese & Japanese recognition models are bundled; the rest are downloadable on demand into `models/ocr/`. See [Localization & OCR Languages](#-localization--ocr-languages).

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

The normalization and fusion live in the frontend (`utils/score.ts`), and each channel can be toggled independently in the UI. Results are scored, ranked (with a stable tie-breaker), and paginated in real time. *Note: the fusion weights are currently fixed constants; relevance sensitivity (loose / standard / strict) is user-adjustable in Settings — see Known Limitations.*

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

> **Note:** The prebuilt package bundles **SigLIP 2 + RapidOCR only** — BGE-M3 is *not* included. The BGE-M3 text model loads only if you download it separately; without it, image & video search still works fully out of the box.
3. You'll see the main interface with the search console
4. Click **Browse** → select a folder containing images
5. Click **Start Indexing** → watch the real-time extraction bus
6. Type a description in the search bar → get millisecond results

---

## 🗺️ Roadmap

> **A note on honesty**: FrameScout is actively developed by a small team. The items below are implemented and functional, but a few are still in a "first version" state — the detailed gaps are listed up front in [Known Limitations](#-known-limitations). Everything marked "planned" is genuinely not built yet.

### v3.2.0 (2026-10-08)

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

### v3.3.0 (Current release — 2026-10-18)

Theme: **logic-search refactor + data-safety closure**. Full record in [CHANGELOG.md](./CHANGELOG.md#330---2026-10-18).

**Search**

- [x] Condition-based search — visible condition rows (keyword / date / batch / file type / seed image) with AND / OR / exclude and per-condition thresholds; input is taken literally — no symbol syntax to learn (`e-mail`, `-5°C`, `会议@2024` all search as typed)
- [x] Smart folders save the **complete condition set** (`criteria_json`, migration M011) and run the same evaluator as the search bar; existing folders keep working unchanged
- [x] Smart folder import / export (JSON) with name dedup — never silently overwrites
- [x] Partition search — source batch / root-path prefix / index-time range filters (also on image search)
- [x] Sort selector — by relevance / index time / filename, with global ordering across pages
- [x] 「试算」 count-only preview button for semantic conditions
- [x] Search explainability — honest ✓ literal-hit bars (no fabricated percentages), adjustable relevance sensitivity (loose / standard / strict), low-score fold toggle, disclosed query rules
- [x] Retired the previous `search_images` / `execute_smart_folder` path — one engine, one semantics

**Data safety**

- [x] In-app database restore — validity check, preview with frame-count comparison, auto-backup before restoring (takes effect on next launch)
- [x] Automatic cleanup policy — event retention & backup rotation with toggles, plus a startup report; `-1` = never
- [x] Text-entry ingest preview — true pre-flight replace counts (`deleted_old_chunks`), no more full-library totals
- [x] Clear error semantics — "found but all decode failed" (`ERR_SCAN_ALL_FAILED`) no longer masquerades as "no new files"

**Experience**

- [x] Scan pause / resume with corrected ETA (pause time excluded), per-file retry from the scan report
- [x] First-screen auto-load of the browse list; search-state session restore (restores the UI, never auto-searches)
- [x] Batch id humanized ("date + file count"), scan-report batch/type filter
- [x] Image search no longer clobbers the query box; `enableOcr` persisted

**i18n**

- [x] Backend error-code contract — stable `CODE|param` codes (`error.rs`) rendered in all 9 locales; no more English sentences leaking into the UI
- [x] zh-CN 「文字识别」 / zh-TW 「文字辨識」 terminology (English keeps OCR)
- [x] All 9 locales fully populated (+92 keys for ja/ru/ko/de/fr/es, +32 new keys this cycle), validated by `scripts/check_i18n_backend.mjs`

**Video**

- [x] FFmpeg fallback decoding — bundled, hash-verified LGPL build for MKV/AVI containers, with a 30s read-timeout watchdog
- [x] Video clip cutting from a hit frame (`cut_video_clip` + `ClipDialog`), output keeps the source container
- [x] Dynamic index RPC timeout + socket linger hygiene — no more zombie worker processes

**Fixes** — seed-image combined conditions returning 0 results, image-search fake 99.9% scores, date quick-ranges pointing into the future, folder rename wiping condition rows, pause timing leaking across scans, `root_path` boundary mismatches, duplicate locale keys, and more.

### v3.4+ (Planned)

- [ ] Faiss integration for N > 50K
- [ ] Automatic port fallback (worker retries the next port if binding fails)
- [ ] FTS5 coverage for OCR / filenames / notes
- [ ] Multi-modal fusion search (text + image simultaneously)
- [ ] Binary PDF/DOCX text extraction
- [ ] Adjustable video frame-extraction limits
- [ ] Seamless in-session database restore (no restart)
- [ ] Server-side counting for condition-row smart folders
- [ ] Whole-library OCR entry (beyond selected files)
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

FrameScout core is licensed under **Apache License 2.0**. See [LICENSE](LICENSE) for details. Third-party components are listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

© 2026 AetherFlow Labs

---

## 💎 Pro License — Complimentary Key

As a **thank-you for your support** — and to make debugging easier — here is a complimentary **FrameScout Pro** license key. It removes the 100-frame trial limit for the full product.

- **Valid until**: 2027-01-01 (version-locked to v3.3.0)
- **License email**: `framescout-offline_ai_search-global`
- **How to activate**: open **Settings → License**, paste the key below, and confirm.

```text
ZnJhbWVzY291dC1vZmZsaW5lX2FpX3NlYXJjaC1nbG9iYWx8RlJBTUVTQ09VVF9QUk9fVjN8My4zLjB8MTc5ODc5MTYyMTo6YvXj2Vqs8SEOhkzJ5KqYRwfKkv2Gbs9uo0ZKA6MXHpDg96Uu7JNcyqjWRozYT01KmL5Vq7SkB6vDRsvdJkG5Dg==
```

> This key is provided as-is for support and debugging purposes. After it expires, the app falls back to the built-in trial (100 indexed frames); the trial limit itself lives in the source and can be modified or removed when building from source under Apache 2.0. For commercial licensing, please contact the author.

🙏 **Thank you for your support!** · 感谢支持！· 感謝支持！· ありがとうございます！· Спасибо за поддержку！· 지원해 주셔서 감사합니다！· Danke für deine Unterstützung! · Merci pour votre soutien ! · ¡Gracias por tu apoyo!

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
