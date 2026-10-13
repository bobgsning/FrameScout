# OCR Multi-language Support

This document explains how text recognition works in the FrameScout inference worker, how multi-language recognition is implemented, and how to add a new language.

> **Current state**: FrameScout OCR ships with **4 built-in languages** (English, 简体中文, 繁體中文, 日本語) and supports **5 downloadable languages** (Русский, 한국어, Deutsch, Français, Español), with 6 more planned. All language metadata lives in a single source of truth — `ocr_manifest.json`.

---

## How OCR Works: a Three-stage Pipeline

FrameScout uses [RapidOCR](https://github.com/RapidAI/RapidOCR) (`rapidocr_onnxruntime`), which is not a single model but a three-stage inference pipeline (`det` → `cls` → `rec`):

| Stage | Role | Language-specific? | Weights |
|-------|------|--------------------|---------|
| `Det` | Text **detection** (locating text regions) | ❌ Universal | `ch_PP-OCRv4_det_infer.onnx` |
| `Cls` | Orientation **classification** (180° correction) | ❌ Universal | `ch_ppocr_mobile_v2.0_cls_infer.onnx` |
| `Rec` | Text **recognition** (image → characters) | ✅ **Language-specific** | `ch_PP-OCRv4_rec_infer.onnx` (default) |

Only the `Rec` stage is language-specific. Multi-language support therefore means **swapping the rec weights only** — `det` and `cls` are shared by all languages and are never re-downloaded.

### Why English + Simplified Chinese share one model

The bundled `ch_PP-OCRv4_rec_infer.onnx` uses a **combined Chinese + English character set**, so a single model recognizes both languages. This is official PP-OCR behavior, not a compromise:

- English letters, digits, punctuation → covered by the character set.
- Simplified Chinese characters (常用汉字) → covered by the same character set.

That is why `en` and `ch_sim` are grouped together internally (see *Grouping* below) and never need a separate model.

> ⚠️ This same model does **not** cover Traditional Chinese characters well, nor Japanese kana/kanji. That is exactly why 繁體中文 and 日本語 ship their own dedicated rec models (`ch_tra`, `ja`).

### The pure-English option (`en_opt`)

For users whose library is English-only, an **English-optimized rec model** is available for download: `en_opt` (`en_PP-OCRv4_rec_mobile.onnx` + `en_dict.txt`).

Why it helps: the CTC recognition head's output classes equal the character-set size. The Chinese+English model has ~6600 classes, so every English letter must be told apart from ~6600 candidates — a pure-English model has only ~95 classes (letters, digits, punctuation), which measurably lowers the confusion rate for visually similar characters (`l`/`1`, `O`/`0`, …). The trade-off: `en_opt` cannot recognize Chinese at all.

`en_opt` and `en`/`ch_sim` are **mutually exclusive in the UI** (the frontend `toggle` auto-deselects one side when the other is picked), because enabling both would OCR English text twice through two models.

### Which model for which scenario

| Scenario | Recommended model(s) | Why |
|----------|----------------------|-----|
| Mixed Chinese + English | `ch_sim` (+ `en`, same model) | `ch_PP-OCRv4_rec` is a combined Chinese+English set — one pass reads both. |
| Pure English | `en_opt` | English-only charset → higher precision, no Chinese to confuse with. |
| Japanese + English | `ja` | The `japan_dict.txt` already contains Latin letters + digits, so it reads embedded English; no extra `en` needed. |
| Traditional Chinese + English | `ch_tra` | `chinese_cht_dict.txt` also includes Latin letters + digits. |
| Korean / Cyrillic / Latin + English | `ko` / `ru` / `de`·`fr`·`es` | Same design — every multilingual dict embeds a base Latin+digit set. |

> The general rule: **every non-English multilingual rec model already includes the Latin alphabet + digits in its character set**, so "language X + embedded English" is handled by the X model alone. Only the "pure English, no Chinese needed" case benefits from the dedicated `en_opt` model.

---

## Language Tiers (three tiers)

Languages are divided into tiers, declared in `ocr_manifest.json`:

| Tier | Languages | Weight source | Deletable |
|------|-----------|---------------|-----------|
| `builtin_wheel` | `en`, `ch_sim` | `rapidocr_onnxruntime` wheel (bundled) | ❌ No |
| `builtin_bundle` | `ch_tra`, `ja` | `models/ocr/<code>/` (bundled at build) | ✅ Yes |
| `downloadable` | `en_opt`, `ru`, `ko`, `de`, `fr`, `es` | `models/ocr/<code>/` (downloaded on demand) | ✅ Yes |
| `planned` | `pt`, `it`, `pl`, `tr`, `vi`, `id` | — (placeholder only) | ❌ No |

- **`builtin_wheel`**: the rec weights live inside the installed wheel (`site-packages/rapidocr_onnxruntime/models/`). Nothing to download, and they cannot be deleted (deleting them would make no sense — they re-appear with the wheel and are shared by `det`/`cls`).
- **`builtin_bundle`**: the rec weights are fetched at build time (`scripts/download_ocr_models.py`) and bundled into the installer, but live as standalone files in `models/ocr/` so the user can delete them.
- **`downloadable`**: the user downloads them from within the app (Settings → OCR recognition languages).
- **`planned`**: listed in the UI as "coming soon", not interactive.

---

## Single Source of Truth: `ocr_manifest.json`

All language metadata is centralized in `src/inference-worker/ocr_manifest.json` and consumed by **three** parties, so nothing drifts out of sync:

1. **Python worker** — `config.py::load_ocr_manifest()` reads it to build the language catalog.
2. **Rust core** — `commands/model_cmd.rs` reads the same file (resolved via `services/paths.rs`, which mirrors the worker's `get_base_path()`) to list / download / delete models.
3. **Frontend** — receives the catalog through the `list_ocr_languages` command.

Each downloadable/bundled language entry carries:

```jsonc
{
  "code": "ja",
  "native": "日本語",
  "tier": "builtin_bundle",
  "deletable": true,
  "rec_model": "japan_PP-OCRv4_rec_mobile.onnx",   // rec weights filename
  "rec_dict": "japan_dict.txt",                     // character dictionary (paired with the model)
  "urls": ["https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.10.0/onnx/..."],
  "dict_urls": ["https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.10.0/paddle/..."],
  "sha256": "e1075a67..."                           // official SHA256, verified on download
}
```

`urls` is an **ordered list**: the official mirror first, self-hosted mirrors appended as fallback. `sha256` is pinned from the official RapidAI/RapidOCR `default_models.yaml` (ModelScope v3.10.0), so downloads are verified against a known-good hash.

> **Source provenance**: model files and hashes are taken from the official [RapidAI/RapidOCR](https://github.com/RapidAI/RapidOCR) model library. Do not substitute unverified mirrors for the `urls`/`sha256` fields.

---

## Grouping: how the worker loads rec weights per language

`OCRManager` (`engines/ocr_engine.py`) caches one `RapidOCR` engine per **group**:

```python
def _group_key(self, code):
    lang = self._catalog.get(code)
    if lang and lang.get("tier") == "builtin_wheel":
        return "default"      # en / ch_sim share the wheel's default weights
    return code               # ch_tra / ja / ru / ... each get their own rec model
```

- `en` + `ch_sim` → group `"default"` → `RapidOCR(det_use_dml=True, cls_use_dml=True, rec_use_dml=True)` (no `rec_model_path`; uses the wheel's `ch_PP-OCRv4_rec_infer.onnx`).
- any other language → group = its code → `RapidOCR(..., rec_model_path=models/ocr/<code>/<rec_model>, rec_keys_path=models/ocr/<code>/<rec_dict>)`.

`rapidocr_onnxruntime` accepts `rec_model_path` / `rec_keys_path` as per-stage kwargs that override its bundled `config.yaml` (verified against v1.4.4). `det`/`cls` are **not** overridden, so they keep using the universal wheel weights.

`ocr_frame()` filters the requested languages to those present in the catalog, then to those with an available model, groups them, runs each group's engine, and merges the per-line results (each line tagged with its group's `lang`).

---

## Dynamic availability

`available_languages()` decides what's usable **at runtime** by checking the disk, so downloading or deleting a language takes effect without restarting the worker:

```python
def available_languages(self):
    for code, lang in self._catalog.items():
        if lang["tier"] == "builtin_wheel":
            available.add(code)                       # always available (in the wheel)
        elif lang["tier"] in ("builtin_bundle", "downloadable"):
            if self._model_files_present(code, lang):
                available.add(code)                   # rec model AND dict must exist on disk
```

`_model_files_present` requires **both** the rec `.onnx` **and** the `.txt` dictionary (when declared). This matters because the v3.10.0 `*_rec_mobile` models do **not** embed their character table — `CTCLabelDecode` raises `"character must not be None"` if the dict is missing, which would otherwise make a language silently fail while still appearing "available".

---

## Directory layout

```text
models/ocr/
├── ch_tra/
│   ├── chinese_cht_PP-OCRv3_rec_mobile.onnx
│   └── chinese_cht_dict.txt
├── ja/
│   ├── japan_PP-OCRv4_rec_mobile.onnx
│   └── japan_dict.txt
├── ru/
│   ├── cyrillic_PP-OCRv3_rec_mobile.onnx
│   └── cyrillic_dict.txt
├── ko/
│   ├── korean_PP-OCRv4_rec_mobile.onnx
│   └── korean_dict.txt
└── de/ fr/ es/                    # share latin_PP-OCRv3_rec_mobile.onnx + latin_dict.txt
```

`de`, `fr`, `es` all use the same `latin_PP-OCRv3_rec_mobile.onnx` (a Latin-script model covers all three), each stored in its own `<code>/` directory for independent download/delete.

---

## Downloading models

Two paths fetch models:

1. **Build time** — `scripts/download_ocr_models.py` (run from the repo root) downloads the `builtin_bundle` languages so they can be bundled into the installer:
   ```powershell
   python scripts/download_ocr_models.py             # ch_tra + ja (built-in)
   python scripts/download_ocr_models.py --all       # also ru/ko/de/fr/es
   python scripts/download_ocr_models.py --lang ja   # one language
   ```
   It verifies SHA256 per file and writes into `src/inference-worker/models/ocr/<code>/`.

2. **Runtime** — the app's `download_ocr_model` command (Rust) downloads on demand with:
   - official mirror first, self-hosted fallback (the `urls` list);
   - HTTP Range resume (writes a `.part`, keeps it on failure);
   - SHA256 verification before atomically renaming `.part` → final file;
   - a disk-space check before writing;
   - throttled `ocr-download-progress` events for the UI progress bar.

`build_nuitka.ps1` then copies `models/` **and** `ocr_manifest.json` next to the worker exe (see BUILD.md).

---

## Adding a new language

1. **Pick the rec weights** from the official RapidAI/RapidOCR model library (ModelScope `v3.10.0` `default_models.yaml`). Note the exact rec model + dict filenames and the official SHA256.
2. **Add an entry** to `ocr_manifest.json` with `code` / `native` / `tier` / `rec_model` / `rec_dict` / `urls` / `dict_urls` / `sha256`.
   - If it's a brand-new language intended for download: use `tier: "downloadable"`, `deletable: true`.
   - If it should ship with the installer: use `tier: "builtin_bundle"`, and add it to `scripts/download_ocr_models.py`'s default selection.
3. **Download it** via `scripts/download_ocr_models.py --lang <code>` (build-time) or through the app (runtime).
4. **Test on real images** — see below.

> ⚠️ PP-OCR `rec` weights are a **paired "model + character dictionary"**. Always obtain both, and keep the dictionary filename matching the manifest, otherwise recognition output will be garbled (or the engine will raise `character must not be None`).

### Verifying a new model on real images

Before shipping a language, run it end-to-end once:

```python
from engines.ocr_engine import OCRManager
import cv2
m = OCRManager("models/ocr")
img = cv2.imread("test.png")
img_rgb = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
print(m.ocr_frame(img_rgb, ["ja"]))
```

Things to confirm:

- The ONNX **input/output node names** of the downloaded rec model match what `rapidocr_onnxruntime` expects (the official `*_rec_mobile.onnx` exports are paddle2onnx-compatible, but a mismatched node name will surface as a runtime error — swap the model filename in the manifest if needed).
- The dictionary file uses the official "one character per line" UTF-8 format (RapidOCR's `CTCLabelDecode.read_character_file` reads line by line).

---

## Notes

- **DirectML compatibility**: swapped `rec` weights still run through DirectML. If a rare operator is incompatible, `_build_engine`'s existing CPU-fallback path downgrades gracefully rather than crashing the worker.
- **Packaging**: language packs live outside the exe in `models/ocr/`; `build_nuitka.ps1` copies `models/` **and** `ocr_manifest.json` next to the exe. See BUILD.md §4.
- **Recognition quality**: most non-Chinese/English languages use PP-OCRv3 rec weights (`*_PP-OCRv3_rec_mobile.onnx`); Japanese uses PP-OCRv4. Refer to official evaluations for expected accuracy.
- **Idempotent download**: `download_ocr_model` skips files that already exist, so a retry after a partial failure only fetches the missing file(s).
