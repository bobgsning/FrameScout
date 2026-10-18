# Contributing to FrameScout

Thanks for stopping by! FrameScout is a small open-source project, and any help is appreciated — whether it's fixing a typo, reporting a bug, or suggesting an idea.

Please read our [AI_POLICY.md](AI_POLICY.md) to understand how AI tools are used in this project.

## How to Contribute

### Report a Bug

Open an [issue](https://github.com/bobgsning/FrameScout/issues) and include:

- What you were doing when the bug occurred
- What you expected to happen vs. what actually happened
- Your OS version and any relevant error messages

That's it. No template required — just describe the problem clearly.

### Suggest a Feature

Open an issue and tell us:

- What problem you're trying to solve
- How you imagine it could work

Even rough ideas are welcome. We'll figure out the details together.

### Submit Code

1. Fork the repo
2. Create a branch: `git checkout -b your-feature-name`
3. Make your changes
4. Test them manually (automated tests are planned but not yet available)
5. If you modified `src/proto/framescout.proto`, regenerate the Python bindings
   (Rust bindings are generated automatically by `build.rs`). Use the bundled
   `protoc` (offline, no install needed) from the **repository root**:

   ```bash
   # Windows (bundled protoc)
   src\proto\protoc.exe --proto_path=src/proto --python_out=src/inference-worker --pyi_out=src/inference-worker src/proto/framescout.proto

   # macOS / Linux (system protoc, https://github.com/protocolbuffers/protobuf/releases)
   protoc -I=./src/proto --python_out=./src/inference-worker --pyi_out=./src/inference-worker ./src/proto/framescout.proto
   ```

   The generated `framescout_pb2.py` / `framescout_pb2.pyi` are tracked in the repository, so this step is only needed when the proto schema changes.
   Remember to bump the `Version:` header in the proto file and add an entry to the changelog there.
6. Push and open a Pull Request

Don't worry too much about commit message conventions. Just write something clear. If AI tools helped you, feel free to mention it in the PR description — but it's not required.
If you'd like, consider using [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) (e.g., feat:, fix:, docs:). It helps generate changelogs automatically.

### Development Setup

Full quick-start with commands: [README.md → Quick Start (Development)](README.md#-quick-start-development).
Packaging the Python worker: [src/inference-worker/BUILD.md](src/inference-worker/BUILD.md).

1. **Prerequisites**: Rust (2021 edition), Node.js 18+, Python 3.10+
2. **Frontend**: `cd src/FrameScout-UI && npm install`
3. **Python worker**: create `.venv` in `src/inference-worker` and install dependencies —
   ⚠️ install `onnxruntime-directml` **first**, then `rapidocr_onnxruntime` with `--no-deps`
   (the install-order trap is documented in BUILD.md §1.2; the wrong order silently
   degrades OCR to CPU)
4. **Models**: `python scripts/download_models.py` (needs a torch + transformers export
   environment; the runtime venv intentionally excludes torch)
5. **Run**: `cd src/FrameScout-UI && npm run tauri dev` (`npm run dev:pro` for the Pro feature)

### Localization (i18n)

The UI ships with **9 languages** (en, zh-CN, zh-TW, ja, ru, ko, de, fr, es), English by
default. Locale files live in `src/FrameScout-UI/src/i18n/locales/*.json`.

- **Backend messages** never contain English sentences: the Rust core returns stable
  error codes (`CODE` or `CODE|param…`, registered in `src-tauri/src/error.rs`), and the
  frontend renders them through each locale's `backend.*` block. When you add a new code,
  add its text to **all 9** locale files.
- **Validate before committing**: `node scripts/check_i18n_backend.mjs` — checks key-set
  parity across the 9 locales, duplicate root keys, and `{0}/{1}/{detail}` placeholder
  alignment.
- **Adding a UI language**: add a locale JSON + register it in `src/i18n/index.ts`
  (`SUPPORTED_LOCALES`).
- **Adding an OCR language**: see
  [src/inference-worker/OCR_MULTILANG.md → Adding a new language](src/inference-worker/OCR_MULTILANG.md#adding-a-new-language)
  (language metadata lives in `ocr_manifest.json`, the single source of truth).

## Code Style

Try to match the existing style of the code you're modifying. If you're unsure, just do your best — We'll clean things up before merging.

- **Rust**: `rustfmt` is preferred but not enforced
- **Python**: Follow basic PEP 8 if possible
- **Vue/TypeScript**: Use `<script setup lang="ts">` like the existing components

## What We Need Help With

These are the areas where contributions would be most valuable right now:

- **Bug fixes** — Always welcome
- **Documentation** — Improving README, fixing typos, adding examples
- **Cross-platform support** — macOS and Linux builds are planned but untested
- **Small UI improvements** — Better layouts, tooltips, responsive tweaks

## What's Not Ready Yet

For large architectural changes or new ML models, please open an issue first to discuss — we'd love to hear your ideas!

## Code of Conduct

Be kind. Be constructive. Assume good intentions.

## Questions?

Open an issue or email <bobgsning@outlook.com>.

> By submitting a pull request, you agree that your contributions will be licensed under the Apache 2.0 license.
