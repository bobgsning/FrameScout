# Changelog

All notable changes to this project will be documented in this file.

## [3.0.1] - 2026-08-07

### Changed

- **AI inference engine overhaul**:
  - Migrated from OpenAI CLIP (512D) to **Google SigLIP 2** (768D) for improved semantic understanding and multilingual support.
  - Switched inference framework from PyTorch to **ONNX Runtime**, with automatic hardware acceleration detection:
    - DirectML (AMD / NVIDIA / Intel GPUs on Windows)
    - CUDA (NVIDIA GPU)
    - CPU (universal fallback)
  - Model path changed to `models/siglip2-base/`; users must re-run `download_models.py`.
- **Frontend low-score collapsible card UI**:
  - Low-confidence results are now collapsed by default, showing a red prompt bar. Click to expand and view details.
  - Added a “▲ Collapse” button at the bottom of expanded cards to re-collapse them.
  - Improved visual styling of the prompt bar (gradient background, rounded corners, hover effect).
- **EasyOCR fully offline**: Model storage directory fixed to `models/easyocr/`; no network dependency.
- **Performance**: ONNX batch inference is ~15–30% faster than PyTorch (depending on GPU).

### Fixed

- Fixed a crash when extracting video frames with NaN FPS values.
- Fixed conflicting display logic between low-score tags and semantic tags.

### Removed

- Removed runtime dependency on PyTorch (still kept in `requirements.txt` for development/debugging purposes).

---

## [3.0.0] - 2026-08-05

> Note: Internal versions 1.x and 2.x existed as closed-source prototypes. This is the first public release.

### Added

- Initial public release of FrameScout Community Edition
- Semantic search via CLIP (text + image embeddings)
- OCR text search via EasyOCR
- Image-to-image search (upload a reference image, find visually similar)
- Video frame indexing (1 FPS extraction, supports MP4/MOV/AVI/MKV)
- Visual clustering with adjustable similarity threshold
- Smart folders (save search conditions, re-run with one click)
- Personal notes with markdown support, searchable
- 100% offline operation — zero network requests
- Hybrid scoring: vector similarity + OCR match + note match + filename match
- FlatVector matrix for fast brute-force search
- Rust + Tauri native desktop app (Windows 10/11)
- Python inference worker (ZMQ + Protobuf IPC)
- Vue 3 frontend with search console, result grid, pagination
- Real-time scan progress with batch processing
- Remove database entries for files that no longer exist on disk (ghost records).

[3.0.1]: https://github.com/bobgsning/FrameScout/releases/tag/v3.0.1
[3.0.0]: https://github.com/bobgsning/FrameScout/releases/tag/v3.0.0
