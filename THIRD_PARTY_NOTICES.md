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

### PP-OCRv4 (RapidOCR weights)

- **Project**: PaddleOCR PP-OCRv4 (det / cls / rec ONNX weights)
- **Copyright**: PaddlePaddle Authors (Baidu)
- **License**: Apache License 2.0
- **Source**: <https://github.com/PaddlePaddle/PaddleOCR>
- **Used for**: optical character recognition (English + Simplified Chinese)

### RapidOCR

- **Project**: RapidOCR (`rapidocr_onnxruntime`)
- **Copyright**: RapidAI contributors
- **License**: Apache License 2.0
- **Source**: <https://github.com/RapidAI/RapidOCR>
- **Used for**: OCR inference pipeline

---

## Bundled Tools

### Protocol Buffers Compiler (`protoc.exe`)

- **Project**: Protocol Buffers
- **Copyright**: Google LLC
- **License**: BSD-3-Clause
- **Source**: <https://github.com/protocolbuffers/protobuf>
- **Used for**: offline Protobuf code generation (`src/proto/`)

---

## Key Libraries

The following libraries are installed via their respective package managers
and are not modified by FrameScout. Their full license texts are available at
the linked sources.

| Component | License |
| --- | --- |
| Tauri | MIT OR Apache-2.0 |
| ZeroMQ (libzmq) | MPL-2.0 |
| ONNX Runtime | MIT |
| OpenCV | Apache-2.0 |
| Vue 3 | MIT |
| vue-i18n | MIT |
| Vite | MIT |
| prost / prost-build | Apache-2.0 |
| rusqlite / SQLite | MIT / Public Domain |
| ed25519-dalek | BSD-3-Clause |
| Transformers | Apache-2.0 |
| NumPy | BSD-3-Clause |
| Pillow | HPND |
| PyZMQ | BSD-3-Clause |
| marked | MIT |

---

## Disclaimer

All third-party components are the property of their respective owners and are
used under their own license terms. FrameScout does not claim ownership of, and
makes no warranty regarding, any third-party component listed above. Refer to
each project's license text for the complete terms.
