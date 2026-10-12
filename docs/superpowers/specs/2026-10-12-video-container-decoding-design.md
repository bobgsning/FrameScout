# FrameScout Video Container Decoding Design

## Scope

Fix FrameScout v3.2.0 video indexing on Windows 11 for MKV, AVI, MP4, and MOV without changing the UI, search behavior, scene detection, semantic indexing, or the existing user database.

## Root cause

The Rust directory scanner and Python worker already whitelist MKV, AVI, MP4, and MOV. The failure occurs after discovery: `video_extractor.py` uses only OpenCV `VideoCapture`. When the installed OpenCV backend cannot open the container or decode its codec, `server.py` catches the exception and returns an empty frame list. Rust consequently returns an indexed-frame count of zero, which the frontend reports as “No new files found”. The message therefore conflates “nothing discovered” with “discovered but decoded zero frames”.

## Design

Keep OpenCV as the primary decoder. Split frame probing from frame selection so both decoders feed the existing scene-change, one-frame-per-second fallback, deduplication, duration, and frame-count rules.

If OpenCV cannot open the file or cannot decode the first frame, invoke an external FFmpeg process. FFmpeg will decode at the configured scene-probe rate into fixed-size RGB24 frames over stdout. A single process is used per video; no per-frame process spawning is allowed. The fallback preserves timestamps and feeds the same scene-selection pipeline as OpenCV.

Resolve FFmpeg in this order:

1. `ffmpeg.exe` beside the packaged `ai_worker.exe`.
2. `ffmpeg` available on `PATH` for source/development runs.

If neither decoder succeeds, raise a descriptive error containing both decoder failures. Existing batch isolation remains: one unreadable video does not abort other files.

## Packaging

Extend `build_nuitka.ps1` to locate FFmpeg during the build and copy `ffmpeg.exe` beside the deployed worker. The standalone application folder retains its current layout and gains only `ai_worker/ffmpeg.exe`.

## Testing

- Assert the Rust and Python extension lists contain MKV, AVI, MP4, and MOV.
- Generate short synthetic MKV, AVI, MP4, and MOV files with FFmpeg.
- Verify normal extraction produces at least one RGB frame for every container.
- Force the OpenCV path to fail and verify the FFmpeg fallback extracts frames.
- Package the worker with Nuitka, build Tauri production, and test the standalone folder on Windows 11.
- Confirm no database migration, deletion, or reset occurs.

## Non-goals

- No UI redesign or copy changes.
- No changes to semantic search, OCR, scene-detection thresholds, or vector storage.
- No replacement of OpenCV when it already decodes the input successfully.
