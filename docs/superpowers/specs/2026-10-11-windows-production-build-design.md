# FrameScout v3.2.0 Windows Production Build Design

## Goal

Build the official Apache-2.0 Global Edition of FrameScout v3.2.0 as a fully local Windows 11 production application and deliver a standalone runnable folder.

## Scope

- Keep the upstream Vue, Tauri/Rust, Python worker, video indexing, scene detection, OCR, notes, clustering, and semantic image/video search unchanged.
- Use the official v3.2.0 tag (`f4a3950`).
- Use the Global Edition, which has no 100-frame trial restriction; do not patch licensing or access controls.
- Download the model set required by the upstream v3.2.0 downloader: SigLIP 2 and BGE-M3. RapidOCR weights remain supplied by its Python package.
- Target Windows 11 x64.

## Build Architecture

Install only the missing Windows build prerequisites. Create a Python virtual environment for the inference worker and preserve the upstream DirectML-first dependency order so supported NVIDIA, AMD, and Intel GPUs can use `DmlExecutionProvider`, with CPU fallback.

Package the worker with the upstream Nuitka PowerShell script in release mode. The script deploys the worker executable, ONNX Runtime DLLs, RapidOCR resources, and downloaded models into Tauri's sidecar directory. Install the frontend dependencies from the lockfile and run the upstream Tauri production build without redesigning or removing features.

## Output

Create a versioned standalone folder under `dist/FrameScout-3.2.0-windows-x64/`. It will contain the runnable application, worker sidecar, models, required runtime assets, `LICENSE`, `THIRD_PARTY_NOTICES.md`, build instructions, installed-dependency inventory, and an error/fix log.

## Validation

- Confirm the checkout exactly matches tag v3.2.0.
- Confirm ONNX Runtime reports `DmlExecutionProvider` on this RTX 4070 Laptop GPU system.
- Run upstream frontend and Rust tests available in the repository.
- Start the packaged application and verify that the worker becomes ready.
- Perform a smoke test covering image indexing, text semantic search, video indexing/scene extraction, and a video-frame search result when practical with local test media.
- Confirm the standalone folder starts independently of the source tree.

## Error Handling

Record build failures and targeted fixes in the final build report. If an upstream script is defective on the current toolchain, apply the smallest build-only correction and document the changed file; do not alter product behavior.
