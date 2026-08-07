# FrameScout Inference Worker

This is the Python inference engine for FrameScout. It runs as a separate process and communicates with the Rust/Tauri core via ZeroMQ + Protobuf.

## What It Does

- **SigLIP 2 (via ONNX Runtime)** — Generates 768-dimensional embeddings for images and text
- **EasyOCR** — Extracts text from images (English, expandable to multi-language)
- **OpenCV** — Extracts frames from videos at 1 FPS (configurable)

## Requirements

- Python 3.10+
- ONNX Runtime (`onnxruntime` or `onnxruntime-directml` for Windows GPU)
- OpenCV (`opencv-python-headless`)
- EasyOCR
- Transformers (for SigLIP 2 processor)
- Protobuf (`protobuf==4.x`)
- ZeroMQ (`pyzmq`)

## Install

```bash
python -m venv .venv
source .venv/bin/activate   # Windows: .venv\Scripts\activate
pip install -r requirements.txt
```

## Model Setup

Models are **not included** in the repository (~1.5 GB total). Download them once before first use:

```bash
# From the repository root
python scripts/download_models.py
```

This places SigLIP 2 ONNX models in `models/siglip2-base/` and EasyOCR models in `models/easyocr/`.

> **Offline note**: After downloading, all models are cached locally. No further network access is required.

## Running

The worker is normally spawned automatically by the Rust core. To run manually:

```bash
python main.py
```

It binds to `tcp://127.0.0.1:5555`. Check the `main.py` for configurable options. Press **Ctrl+C** to stop.

## Hardware Acceleration

ONNX Runtime automatically selects the best available execution provider:

- **DirectML** – AMD, NVIDIA, Intel GPUs (Windows)
- **CUDA** – NVIDIA GPU
- **CPU** – universal fallback

No manual configuration needed.

## Troubleshooting

Logs are written to `%TEMP%\FrameScout_Global\ai_worker_*.log`. For verbose output, set `DEBUG=1` environment variable before starting.

## Protocol

See [../proto/search.proto](../proto/search.proto) for message definitions. The generated `search_pb2.py` is tracked in the repository.

## License

Apache 2.0. See [../../LICENSE](../../LICENSE).

---
