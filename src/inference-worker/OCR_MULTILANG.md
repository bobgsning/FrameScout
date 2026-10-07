# OCR Multi-language Support

This document explains how text recognition works in the FrameScout inference worker and how to add languages.

> **Current state**: FrameScout OCR ships with **English + Simplified Chinese** out of the box. Other languages are not yet bundled.

## How OCR Works: a Three-stage Pipeline

FrameScout uses [RapidOCR](https://github.com/RapidAI/RapidOCR) (`rapidocr_onnxruntime`), which is not a single model but a three-stage inference pipeline (`det` → `cls` → `rec`):

| Stage | Role | Language-specific? | Weights |
|-------|------|--------------------|---------|
| `Det` | Text **detection** (locating text regions) | ❌ Universal | `ch_PP-OCRv4_det_infer.onnx` |
| `Cls` | Orientation **classification** (180° correction) | ❌ Universal | `ch_ppocr_mobile_v2.0_cls_infer.onnx` |
| `Rec` | Text **recognition** (image → characters) | ✅ **Language-specific** | `ch_PP-OCRv4_rec_infer.onnx` |

The `Rec` stage's character set determines which languages can be recognized. The bundled `ch_PP-OCRv4_rec` model uses a combined Chinese + English character set, which is why both languages work out of the box.

### Project-side configuration

```python
# engines/ocr_engine.py
OCR_ENGINE_NAME = "rapidocr"
OCR_MODEL_NAME = "ppocrv4"
SUPPORTED_LANGUAGES = {"en", "ch_sim"}
```

```python
# config.py
DEFAULT_OCR_LANGS = ["en"]
```

All three stage weights ship inside the `rapidocr_onnxruntime` wheel (bundled at package time), so nothing needs to be downloaded for the default languages. `models/ocr/` is a placeholder directory reserved for future downloaded language packs.

## Adding a Language: Swap the Rec Weights

Because `det` and `cls` are universal, adding a new language only means pointing `Rec` at a different recognition model. `rapidocr_onnxruntime` accepts per-stage kwargs that override its `config.yaml`:

```python
RapidOCR(
    det_use_dml=True, cls_use_dml=True, rec_use_dml=True,
    rec_model_path=r"...\japan_PP-OCRv3_rec_infer.onnx",   # target language's rec model
    # rec_keys_path=r"...\japan_dict.txt",                 # only if the charset isn't embedded in the ONNX
)
```

> ⚠️ PP-OCR `rec` weights are a paired "model + character dictionary". Some distributions embed the dictionary in the ONNX; others ship a separate `dict.txt`. Always obtain the model and its dictionary together, otherwise recognition output will be garbled.

### Suggested language-pack layout

```text
model_storage_dir/
  ppocrv4_rec/
    japan_PP-OCRv3_rec_infer.onnx
    japan_dict.txt
    korean_PP-OCRv3_rec_infer.onnx
    korean_dict.txt
    ...
```

The `OCRManager` already accepts a `model_storage_dir` parameter reserved for this purpose.

### Where to get language packs

1. The official [RapidOCR](https://github.com/RapidAI/RapidOCR) repository — per-language PP-OCR ONNX weights.
2. PaddleOCR's multilingual models (PP-OCRv3/v4), converted to ONNX with `paddle2onnx`.
3. RapidOCR multilingual mirrors on ModelScope / Hugging Face.

Download URLs change over time, so consult the RapidOCR repository's current `models` manifest when adding a language. The `det` weights (`ch_PP-OCRv4_det`) are universal — do not re-download them.

## Notes

- **DirectML compatibility**: swapped `rec` weights still run through DirectML. If a rare operator is incompatible, the existing CPU-fallback path in `_build_engine` downgrades gracefully rather than crashing the worker.
- **Packaging**: language packs live outside the exe in `model_storage_dir`. `build_nuitka.ps1` copies the `models/` directory; if language packs are stored elsewhere, copy them alongside in the packaging step too.
- **Recognition quality**: most non-Chinese/English languages use PP-OCRv3 weights. Refer to official evaluations for expected accuracy.
