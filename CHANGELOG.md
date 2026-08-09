# Changelog

All notable changes to this project will be documented in this file.

## [3.0.2] - 2026-08-09

### Added v3.0.2

- **“Show All” mode**: New backend command `get_all_files` returns every indexed file (no pagination). Frontend adds a “📋 Show All” button in the pagination bar and a quick-access button in the search toolbar. While in “Show All” mode, a hint bar displays the total count and a “📄 Paginated View” button to return to paginated browsing.
- **Protobuf `index_time` field** (`FrameResult.index_time`, `double`): Python worker now stamps each indexed file with the current Unix timestamp.
- **Database column `index_time`**: `frame_vectors` table gains `index_time REAL`. Existing databases are automatically migrated via `ALTER TABLE`.
- **Backend command `list_all_files`**: Paginated listing ordered by `index_time DESC`, used by the browse-mode pagination.

### Changed v3.0.2

- **Browse mode sorting**: `list_all_files` now sorts by `index_time DESC` instead of `timestamp DESC`. Newly indexed files consistently appear first.
- **`AppState.memory_db` upgraded from `Mutex` to `RwLock`**: concurrent reads allowed, writes exclusive – improves responsiveness during parallel searches.
- **`request_vector` creates a fresh ZMQ REQ socket per call**: eliminates shared-socket contention and simplifies reconnection logic.
- **`clean_ghosts` returns structured result** (`CleanResult { removed_count, removed_paths }`): frontend precisely removes ghost entries from the current result set.
- **Scan progress handler**: `new_files` are always queued into `incomingFiles` and displayed via banner – never injected directly into `results` to avoid page disruption.

### Fixed v.3.0.2

- **New files not appearing after clicking the “📥 N new images” banner**: root cause was that images had `timestamp = 0.0`, making `ORDER BY timestamp DESC` degenerate to physical storage order. Solved by introducing `index_time`.
- **Stale responses overwriting newer ones in `performSearch`**: added `searchRequestId` counter – only the latest request’s result is applied.
- **`clean_ghosts` clearing all results in browse mode**: now removes only the actual ghost paths and reloads the current page.
- **`acceptIncomingFiles` forcefully exiting search/clustering context**: now transitions to browse mode (first page) or refreshes “Show All” mode accordingly.
- **`changePage` allowing invalid page numbers**: clamped to `[1, totalPages]`.
- **`clearImageSearch` clearing results even when not in image-search mode**: early return guard added.
- **`toggleClustering` failing to restore previous results on exit**: now correctly saves and restores `fullResultsCache`.

### Improved v.3.0.2

- **Frontend debouncing**: rapid page turns or search switches only apply the last response.
- **`scan_folder` uses `RwLock` semantics (`read()` / `write()`) for `memory_db`**.
- **“Show All” mode integrates seamlessly**: works with `acceptIncomingFiles`, `cleanGhosts`, and clustering (exits to paginated view when searching).

[3.0.2]: https://github.com/bobgsning/FrameScout/releases/tag/v3.0.2

## [3.0.1] - 2026-08-07

### Changed v3.0.1

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

### Fixed v3.0.1

- Fixed a crash when extracting video frames with NaN FPS values.
- Fixed conflicting display logic between low-score tags and semantic tags.

### Removed v3.0.1

- Removed runtime dependency on PyTorch (still kept in `requirements.txt` for development/debugging purposes).

---

## [3.0.0] - 2026-08-05

> Note: Internal versions 1.x and 2.x existed as closed-source prototypes. This is the first public release.

### Added v3.0.0

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
