# FrameScout Inference Worker

This is the Python inference engine for FrameScout. It runs as a separate process and communicates with the Rust/Tauri core via ZeroMQ + Protobuf.

## What It Does

- **CLIP ViT-B/32** — Generates 512-dimensional embeddings for images and text
- **EasyOCR** — Extracts text from images (English, expandable to multi-language)
- **OpenCV** — Extracts frames from videos at configurable FPS

## Requirements

- Python 3.10+
- PyTorch 2.x
- OpenCV (`opencv-python-headless`)
- EasyOCR
- Protobuf (`protobuf==4.x`)
- ZeroMQ (`pyzmq`)

## Install

```bash
python -m venv .venv
source .venv/bin/activate   # Windows: .venv\Scripts\activate
pip install -r requirements.txt
```

## Troubleshooting

Logs are printed to stdout. For more verbose output, set the environment variable before starting:

**Windows (PowerShell):**

```powershell
$env:DEBUG="1"; python main.py
```

**Linux/macOS:**

```bash
DEBUG=1 python main.py
```

**Windows (CMD):**

```cmd
set DEBUG=1 && python main.py
```

## Model Setup

FrameScout requires two sets of models: CLIP and EasyOCR. Because of their size (~1.4 GB total), they are not included in the repository and must be downloaded once before first use.

### Option A: Use the download script (recommended)

```bash
python ../scripts/download_models.py
```

This will download both CLIP and EasyOCR models and place them in the correct directories.

### Option B: Manual download

**CLIP model** – Download from Hugging Face:
[https://huggingface.co/openai/clip-vit-base-patch32](https://huggingface.co/openai/clip-vit-base-patch32)

Place all files in `models/openai-clip/`:

```text
models/openai-clip/
├── config.json
├── preprocessor_config.json
├── pytorch_model.bin (or model.safetensors)
└── vocab.json (if applicable)
```

**EasyOCR models** – These are auto-downloaded on first run if not present. For a fully offline setup, manually download:

- `craft_mlt_25k.pth` (text detection)
- `english_g2.pth` (English recognition)

Place them in `models/easyocr/`.

> **Note**: After the initial download, all models are cached locally. No further network access is required — FrameScout runs completely offline from that point onward.

## Running

The worker is normally spawned automatically by the Rust core. To run manually:

```bash
python main.py
```

By default, it binds to `tcp://127.0.0.1:5555`. Check the `main.py` for configurable options. Press **Ctrl+C** to stop it gracefully.

## Protocol

See ../proto/search.proto for the full message definitions. The `search_pb2.py` file in this directory is generated from `search.proto` and is tracked in the repository.

## License

This component is part of the FrameScout repository and is licensed under the same terms as the main project (Apache 2.0). See ../../LICENSE for details.

---
