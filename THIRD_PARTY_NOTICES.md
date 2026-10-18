# Third-Party Notices

This file lists third-party components that FrameScout uses or redistributes,
together with their licenses and copyright holders. It is provided to satisfy
the attribution and license-notice requirements of those projects.

---

## Redistributed AI Model Weights

FrameScout's release packages bundle the following pre-trained model weights.
These are redistributed **unmodified**; their original licenses require that
this notice accompany the distribution.

### SigLIP 2

- **Project**: `google/siglip2-base-patch16-256`
- **Copyright**: Google LLC
- **License**: Apache License 2.0
- **Source**: <https://huggingface.co/google/siglip2-base-patch16-256>
- **Used for**: visual / text semantic embeddings (768-dim)

### BGE-M3

- **Project**: `BAAI/bge-m3`
- **Copyright**: Beijing Academy of Artificial Intelligence (BAAI)
- **License**: MIT License
- **Source**: <https://huggingface.co/BAAI/bge-m3>
- **Used for**: dense text embeddings (1024-dim)

### PP-OCR ONNX weights (via RapidOCR)

- **Project**: PaddleOCR PP-OCR ONNX weights (det / cls / rec), obtained through the
  RapidAI/RapidOCR model library
- **Copyright**: PaddlePaddle Authors (Baidu)
- **License**: Apache License 2.0
- **Source**: <https://github.com/RapidAI/RapidOCR> (model files & SHA-256 hashes pinned from
  ModelScope `RapidAI/RapidOCR` v3.10.0)
- **Used for**: optical character recognition — 10 recognition languages
  (English, 简体中文, 繁體中文, 日本語 built-in; Русский, 한국어, Deutsch, Français, Español
  and an English-optimized variant downloadable on demand)
- **Notes**: the `det` / `cls` stages are universal PP-OCRv4 weights shared by all languages;
  the language-specific `rec` stage is a mix of PP-OCRv3 and PP-OCRv4 mobile weights
  (see `src/inference-worker/ocr_manifest.json` for the exact per-language files and hashes)

### RapidOCR

- **Project**: RapidOCR (`rapidocr_onnxruntime`)
- **Copyright**: RapidAI contributors
- **License**: Apache License 2.0
- **Source**: <https://github.com/RapidAI/RapidOCR>
- **Used for**: OCR inference pipeline

---

## Redistributed Binaries (shipped inside release packages)

### FFmpeg (`ffmpeg.exe`)

- **Project**: FFmpeg
- **Copyright**: FFmpeg developers
- **License**: GNU Lesser General Public License v2.1 or later (LGPL v2.1+) — distributed as a
  **LGPL build** (built with `--disable-gpl --disable-nonfree`; the build script verifies the
  build configuration and rejects GPL/nonfree variants)
- **Source**: <https://github.com/BtbN/FFmpeg-Builds> (upstream: <https://ffmpeg.org>)
- **Used for**: fallback video frame extraction (when OpenCV cannot open/decode a container,
  e.g. some MKV/AVI codecs) and cutting short video clips from search hits
- **Full license text**: see `COPYING.LGPLv2.1` in the distribution root

> FrameScout invokes FFmpeg as a **separate process** (`ffmpeg.exe`) rather than linking it
> into the application. This is aggregation, not linking, so the LGPL does not impose copyleft
> on FrameScout itself. You may replace this component by placing your own `ffmpeg.exe` beside
> the worker executable (`ai_worker/ffmpeg.exe`).

### Microsoft Edge WebView2 Runtime

- **Project**: Microsoft Edge WebView2 Runtime
- **Copyright**: Microsoft Corporation
- **License**: Microsoft proprietary — [WebView2 redistribution terms](https://developer.microsoft.com/microsoft-edge/webview2/concepts/distribution)
- **Source**: <https://developer.microsoft.com/microsoft-edge/webview2/>
- **Used for**: the webview that renders the FrameScout UI (Tauri's rendering surface)
- **Notes**: the Windows installer is built with `webviewInstallMode: "offlineInstaller"`,
  which bundles the WebView2 standalone installer. Redistribution is governed by Microsoft's
  own distribution terms, not this project's license.

---

## Key Libraries

The following libraries are installed via their respective package managers
and are not modified by FrameScout. Their full license texts are available at
the linked sources.

### Rust crates (`src-tauri/`)

| Component | License |
| --- | --- |
| Tauri | MIT OR Apache-2.0 |
| tauri-build / tauri-plugin-dialog / tauri-plugin-opener | MIT OR Apache-2.0 |
| prost / prost-build | Apache-2.0 |
| rusqlite / SQLite | MIT / Public Domain |
| ed25519-dalek | BSD-3-Clause |
| ZeroMQ (libzmq bindings) | MPL-2.0 |
| serde / serde_json | MIT OR Apache-2.0 |
| reqwest | MIT OR Apache-2.0 |
| sha2 | Apache-2.0 OR MIT |
| sysinfo | MIT |
| walkdir | MIT OR Apache-2.0 |
| dirs | MIT OR Apache-2.0 |
| base64 / hex | MIT OR Apache-2.0 |

### JavaScript / TypeScript (`src/FrameScout-UI/`)

| Component | License |
| --- | --- |
| Vue 3 | MIT |
| vue-i18n | MIT |
| Vite | MIT |
| @vitejs/plugin-vue | MIT |
| @tauri-apps/api / cli / plugin-dialog / plugin-opener | MIT OR Apache-2.0 |
| TypeScript | Apache-2.0 |
| vue-tsc | MIT |
| marked | MIT |

### Python (`src/inference-worker/`)

| Component | License |
| --- | --- |
| ONNX Runtime | MIT |
| OpenCV | Apache-2.0 |
| Transformers | Apache-2.0 |
| sentencepiece | Apache-2.0 |
| NumPy | BSD-3-Clause |
| Pillow | HPND |
| PyZMQ | BSD-3-Clause |
| protobuf (incl. `framescout_pb2.py`) | BSD-3-Clause |

---

## Development & Build Tools (not shipped in release packages)

These tools are used during development and packaging only; they are **not**
redistributed inside FrameScout release packages.

| Component | License | Used for |
| --- | --- | --- |
| Protocol Buffers compiler (`protoc.exe`) | BSD-3-Clause | offline Protobuf code generation (`src/proto/`) |
| Nuitka / zstandard | Apache-2.0 / BSD-3-Clause | Python worker packaging |
| PyInstaller | GPL-2.0 with exception (legacy fallback) | Python worker packaging (legacy) |
| grpcio-tools | Apache-2.0 | cross-platform Protobuf code generation |

---

## Disclaimer

All third-party components are the property of their respective owners and are
used under their own license terms. FrameScout does not claim ownership of, and
makes no warranty regarding, any third-party component listed above. Refer to
each project's license text for the complete terms.
