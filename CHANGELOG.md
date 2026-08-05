# Changelog

All notable changes to this project will be documented in this file.

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

[3.0.0]: https://github.com/bobgsning/FrameScout/releases/tag/v3.0.0
