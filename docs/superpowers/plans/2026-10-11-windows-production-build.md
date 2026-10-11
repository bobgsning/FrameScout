# FrameScout v3.2.0 Windows Production Build Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the official FrameScout v3.2.0 Global Edition into a tested standalone Windows 11 x64 folder.

**Architecture:** Keep upstream product code unchanged. Build the Python inference worker as a Nuitka onefile sidecar with DirectML and external models, then build the Vue/Tauri application and assemble a portable folder containing all runtime assets and build records.

**Tech Stack:** Python 3.11, Nuitka, ONNX Runtime DirectML, SigLIP 2, BGE-M3, Node.js/npm, Vue 3, Rust, Tauri 2, MSVC Build Tools.

## Global Constraints

- Use official tag `v3.2.0` at upstream commit `f4a3950`.
- Target Windows 11 x64.
- Do not patch licensing or access controls; use the unrestricted Apache-2.0 Global Edition.
- Preserve video indexing, scene detection, semantic image/video search, and all upstream functionality.
- Use GPU acceleration through `DmlExecutionProvider` when supported, with CPU fallback.
- Package the inference worker with Nuitka.
- Deliver `dist/FrameScout-3.2.0-windows-x64/` as a standalone runnable folder.

---

### Task 1: Verify source and install build toolchains

**Files:**
- Inspect: `README.md`
- Inspect: `src/inference-worker/BUILD.md`
- Create: `build-records/toolchain-versions.txt`

**Interfaces:**
- Consumes: Windows 11 host, FrameScout v3.2.0 clone.
- Produces: Node/npm, Python, Rust/Cargo, MSVC, WebView2, and version inventory available to later tasks.

- [ ] **Step 1: Verify the source revision and clean product tree**

```powershell
git describe --tags --exact-match
git rev-parse 'v3.2.0^{}'
git status --short
```

Expected: tag `v3.2.0`, peeled commit `f4a3950ce2f876c844a7b19c24450b1f016641d4`; only approved build documentation may differ from upstream.

- [ ] **Step 2: Install missing Windows toolchains**

```powershell
winget install --id Rustlang.Rustup --exact --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended" --accept-package-agreements --accept-source-agreements
```

Expected: Rust stable MSVC and Visual Studio C++ Build Tools are installed. Skip a command when the matching toolchain is already usable.

- [ ] **Step 3: Record exact toolchain versions**

```powershell
New-Item -ItemType Directory -Force build-records | Out-Null
@(
  "OS: $((Get-ComputerInfo).WindowsProductName) $((Get-ComputerInfo).OsBuildNumber)",
  "Node: $(node --version)",
  "npm: $(npm --version)",
  "Python: $(python --version)",
  "Rust: $(rustc --version)",
  "Cargo: $(cargo --version)"
) | Set-Content -Encoding utf8 build-records/toolchain-versions.txt
```

Expected: `build-records/toolchain-versions.txt` contains each detected version.

### Task 2: Prepare inference environment and models

**Files:**
- Inspect: `requirements.txt`
- Inspect: `scripts/download_models.py`
- Create: `src/inference-worker/.venv/`
- Create: `src/inference-worker/models/siglip2-base/`
- Create: `src/inference-worker/models/bge-m3/`
- Create: `build-records/python-packages.txt`

**Interfaces:**
- Consumes: Python 3.11 and upstream requirements.
- Produces: working DirectML virtual environment plus local SigLIP 2 and BGE-M3 ONNX assets.

- [ ] **Step 1: Create the worker virtual environment and install DirectML safely**

```powershell
python -m venv src/inference-worker/.venv
src/inference-worker/.venv/Scripts/python.exe -m pip install --upgrade pip
src/inference-worker/.venv/Scripts/python.exe -m pip install "onnxruntime-directml>=1.24.4"
src/inference-worker/.venv/Scripts/python.exe -m pip install --no-deps "rapidocr_onnxruntime>=1.3.23"
src/inference-worker/.venv/Scripts/python.exe -m pip install -r requirements.txt
src/inference-worker/.venv/Scripts/python.exe -m pip install nuitka zstandard
```

Expected: imports succeed without a second standard `onnxruntime` distribution replacing DirectML.

- [ ] **Step 2: Verify the GPU provider**

```powershell
src/inference-worker/.venv/Scripts/python.exe -c "import onnxruntime as ort; print(ort.get_available_providers())"
```

Expected: output includes `DmlExecutionProvider` and `CPUExecutionProvider`.

- [ ] **Step 3: Install export-only model dependencies and download/export SigLIP 2**

```powershell
src/inference-worker/.venv/Scripts/python.exe -m pip install "torch>=2" onnx packaging
src/inference-worker/.venv/Scripts/python.exe scripts/download_models.py
```

Expected: `siglip2_vision.onnx`, `siglip2_text.onnx`, tokenizer/processor files, and a successful RapidOCR self-check.

- [ ] **Step 4: Download/export BGE-M3**

```powershell
src/inference-worker/.venv/Scripts/python.exe scripts/download_models.py --bge
```

Expected: `src/inference-worker/models/bge-m3/bge_m3_dense.onnx` and tokenizer files exist.

- [ ] **Step 5: Record Python dependencies**

```powershell
src/inference-worker/.venv/Scripts/python.exe -m pip freeze | Set-Content -Encoding utf8 build-records/python-packages.txt
```

Expected: reproducible installed-package inventory is saved.

### Task 3: Build and validate the Nuitka worker

**Files:**
- Execute: `src/inference-worker/build_nuitka.ps1`
- Create: `src/inference-worker/dist_nuitka/ai_worker.exe`
- Create: `src/FrameScout-UI/src-tauri/bin/ai_worker/`

**Interfaces:**
- Consumes: DirectML environment and downloaded model directories from Task 2.
- Produces: Tauri-compatible `ai_worker.exe`, runtime libraries, and model assets.

- [ ] **Step 1: Run worker import checks**

```powershell
src/inference-worker/.venv/Scripts/python.exe -c "import config, server, framescout_pb2; print('worker imports OK')"
```

Expected: `worker imports OK`.

- [ ] **Step 2: Build and deploy the release worker**

```powershell
& src/inference-worker/.venv/Scripts/Activate.ps1
& src/inference-worker/build_nuitka.ps1 -NoTorch
```

Expected: `src/FrameScout-UI/src-tauri/bin/ai_worker/ai_worker.exe` plus sibling `models/`.

- [ ] **Step 3: Smoke-test the packaged worker**

```powershell
$worker = Start-Process -FilePath src/FrameScout-UI/src-tauri/bin/ai_worker/ai_worker.exe -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 20
Test-NetConnection -ComputerName 127.0.0.1 -Port 5555
Stop-Process -Id $worker.Id
```

Expected: `TcpTestSucceeded : True`; worker terminates cleanly.

### Task 4: Build the Tauri production application

**Files:**
- Consume: `src/FrameScout-UI/package-lock.json`
- Create: `src/FrameScout-UI/node_modules/`
- Create: `src/FrameScout-UI/src-tauri/target/release/`
- Create: `build-records/npm-packages.txt`

**Interfaces:**
- Consumes: worker sidecar from Task 3 and Rust/MSVC/Node toolchains from Task 1.
- Produces: signed-or-unsigned local production executable and Tauri bundle artifacts.

- [ ] **Step 1: Install locked frontend dependencies**

```powershell
Push-Location src/FrameScout-UI
npm ci
npm ls --all | Out-File -Encoding utf8 ../../build-records/npm-packages.txt
Pop-Location
```

Expected: `npm ci` completes and the dependency tree is recorded.

- [ ] **Step 2: Run focused frontend validation**

```powershell
Push-Location src/FrameScout-UI
npm run type-check
npm run build
Pop-Location
```

Expected: Vue type-check and Vite production build pass.

- [ ] **Step 3: Run Rust tests**

```powershell
Push-Location src/FrameScout-UI/src-tauri
cargo test --locked
Pop-Location
```

Expected: all discovered Rust tests pass.

- [ ] **Step 4: Build Tauri production artifacts**

```powershell
Push-Location src/FrameScout-UI
npm run tauri:build
Pop-Location
```

Expected: release executable and configured Windows bundles appear under `src/FrameScout-UI/src-tauri/target/release/`.

### Task 5: Assemble and verify the standalone folder

**Files:**
- Create: `dist/FrameScout-3.2.0-windows-x64/`
- Create: `dist/FrameScout-3.2.0-windows-x64/BUILD-INSTRUCTIONS.md`
- Create: `dist/FrameScout-3.2.0-windows-x64/BUILD-REPORT.md`
- Copy: `LICENSE`
- Copy: `THIRD_PARTY_NOTICES.md`

**Interfaces:**
- Consumes: Tauri release application and worker deployment from Tasks 3-4.
- Produces: self-contained delivery folder and final audit records.

- [ ] **Step 1: Copy production runtime files into the delivery folder**

```powershell
$out = 'dist/FrameScout-3.2.0-windows-x64'
New-Item -ItemType Directory -Force $out | Out-Null
Copy-Item src/FrameScout-UI/src-tauri/target/release/FrameScout.exe $out
Copy-Item -Recurse src/FrameScout-UI/src-tauri/bin/ai_worker "$out/ai_worker"
Copy-Item LICENSE,THIRD_PARTY_NOTICES.md $out
Copy-Item -Recurse build-records "$out/build-records"
```

Expected: application, worker, models, notices, and dependency inventories are present. Adjust only the executable filename if Cargo emits the configured binary under a different case/name.

- [ ] **Step 2: Write reproducible build instructions and error log**

Create `BUILD-INSTRUCTIONS.md` from the verified commands in Tasks 1-4. Create `BUILD-REPORT.md` listing source tag/commit, output path, installed dependencies, validation results, and every encountered error with its applied fix.

- [ ] **Step 3: Verify delivery-folder independence**

```powershell
$app = Start-Process -FilePath 'dist/FrameScout-3.2.0-windows-x64/FrameScout.exe' -PassThru
Start-Sleep -Seconds 30
Test-NetConnection -ComputerName 127.0.0.1 -Port 5555
Stop-Process -Id $app.Id
```

Expected: application starts from the delivery folder and its worker listens on port 5555.

- [ ] **Step 4: Run functional smoke tests**

Index a small local sample containing at least one image and one short scene-changing video. Verify text semantic search returns the image, video indexing produces searchable frame timestamps, scene extraction completes, and opening a video result seeks to the matching timestamp.

- [ ] **Step 5: Audit output and commit build documentation**

```powershell
Get-ChildItem -Recurse dist/FrameScout-3.2.0-windows-x64 | Select-Object FullName,Length
git status --short
git add docs/superpowers/plans/2026-10-11-windows-production-build.md dist/FrameScout-3.2.0-windows-x64/BUILD-INSTRUCTIONS.md dist/FrameScout-3.2.0-windows-x64/BUILD-REPORT.md
git commit -m "build: document Windows production package"
```

Expected: final folder is complete and build documentation is committed without unrelated source changes.
