# Video Container Decoding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reliably index MKV, AVI, MP4, and MOV videos on Windows 11 by retaining OpenCV as the primary decoder and adding a packaged FFmpeg fallback.

**Architecture:** `video_extractor.py` will separate raw frame probing from the existing scene-selection policy. OpenCV supplies probe frames first; if it cannot open or decode the first frame, one FFmpeg subprocess supplies fixed-size RGB24 probe frames to the same selection pipeline. The Nuitka build copies `ffmpeg.exe` beside `ai_worker.exe`.

**Tech Stack:** Python 3.11, OpenCV, FFmpeg, unittest, Nuitka, Rust/Tauri 2, PowerShell.

## Global Constraints

- Target FrameScout v3.2.0 on Windows 11 x64.
- Do not change the UI, search functions, scene-detection thresholds, or vector storage.
- Do not delete, reset, migrate, or otherwise modify the user's existing database.
- Preserve OpenCV as the primary decoder and use FFmpeg only as fallback.
- Preserve MKV, AVI, MP4, MOV, WEBM, FLV, M4V, and TS extension support.

---

### Task 1: Lock extension support with tests

**Files:**
- Modify: `src/FrameScout-UI/src-tauri/src/constants.rs`
- Create: `src/inference-worker/tests/test_video_extractor.py`

**Interfaces:**
- Consumes: `VIDEO_EXTENSIONS` in Rust and `is_video_file(path: str) -> bool` in Python.
- Produces: regression coverage proving MKV, AVI, MP4, and MOV remain discoverable.

- [ ] **Step 1: Add the Rust regression test**

```rust
#[cfg(test)]
mod video_extension_tests {
    use super::VIDEO_EXTENSIONS;

    #[test]
    fn required_video_containers_are_scannable() {
        for extension in ["mkv", "avi", "mp4", "mov"] {
            assert!(VIDEO_EXTENSIONS.contains(&extension));
        }
    }
}
```

- [ ] **Step 2: Add the Python extension regression test**

```python
import unittest

from media.video_extractor import is_video_file


class VideoExtensionTests(unittest.TestCase):
    def test_required_video_containers_are_supported(self):
        for extension in ("mkv", "avi", "mp4", "mov"):
            with self.subTest(extension=extension):
                self.assertTrue(is_video_file(f"sample.{extension}"))
                self.assertTrue(is_video_file(f"sample.{extension.upper()}"))
```

- [ ] **Step 3: Run focused tests**

Run:

```powershell
cd src\FrameScout-UI\src-tauri
cargo test required_video_containers_are_scannable --locked
cd ..\..\inference-worker
.\.venv\Scripts\python.exe -m unittest tests.test_video_extractor.VideoExtensionTests -v
```

Expected: both tests pass, proving the problem is not the v3.2.0 extension lists.

- [ ] **Step 4: Commit**

```powershell
git add src/FrameScout-UI/src-tauri/src/constants.rs src/inference-worker/tests/test_video_extractor.py
git commit -m "test: lock supported video containers"
```

### Task 2: Add a shared decoder pipeline with FFmpeg fallback

**Files:**
- Modify: `src/inference-worker/media/video_extractor.py`
- Modify: `src/inference-worker/tests/test_video_extractor.py`

**Interfaces:**
- Consumes: `config.get_base_path()`, existing extraction constants, `cv2.VideoCapture`, and `ffmpeg.exe`.
- Produces: `_iter_opencv_frames(path, probe_fps, max_duration)`, `_iter_ffmpeg_frames(path, probe_fps, max_duration)`, and unchanged public `extract_video_frames(...)` generator behavior.

- [ ] **Step 1: Add a failing forced-fallback test**

```python
from pathlib import Path
from unittest import mock

from media.video_extractor import extract_video_frames


class ClosedCapture:
    def isOpened(self):
        return False

    def release(self):
        pass


def test_ffmpeg_fallback_when_opencv_cannot_open(self):
    sample = self.samples["mkv"]
    with mock.patch("media.video_extractor.cv2.VideoCapture", return_value=ClosedCapture()):
        frames = list(extract_video_frames(str(sample), max_duration=2, max_frames=4))
    self.assertGreater(len(frames), 0)
    self.assertEqual(frames[0][1].shape[2], 3)
```

- [ ] **Step 2: Run it and verify the current implementation fails**

Run:

```powershell
.\.venv\Scripts\python.exe -m unittest tests.test_video_extractor.VideoDecodeTests.test_ffmpeg_fallback_when_opencv_cannot_open -v
```

Expected: error `Cannot open video`.

- [ ] **Step 3: Add FFmpeg resolution and raw-frame decoding**

Implement in `video_extractor.py`:

```python
import shutil
import subprocess
from itertools import chain

from config import get_base_path

FFMPEG_FRAME_SIZE = 256


def _resolve_ffmpeg() -> str:
    packaged = os.path.join(get_base_path(), "ffmpeg.exe")
    if os.path.isfile(packaged):
        return packaged
    system = shutil.which("ffmpeg")
    if system:
        return system
    raise FileNotFoundError("FFmpeg fallback is unavailable: ffmpeg.exe was not packaged and ffmpeg is not on PATH")


def _iter_ffmpeg_frames(video_path: str, probe_fps: float, max_duration: float):
    size = FFMPEG_FRAME_SIZE
    frame_bytes = size * size * 3
    vf = (
        f"fps={probe_fps},scale={size}:{size}:force_original_aspect_ratio=decrease,"
        f"pad={size}:{size}:(ow-iw)/2:(oh-ih)/2:black"
    )
    command = [
        _resolve_ffmpeg(), "-nostdin", "-v", "error", "-i", video_path,
        "-t", str(max_duration), "-vf", vf,
        "-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1",
    ]
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    yielded = 0
    try:
        while True:
            data = process.stdout.read(frame_bytes)
            if not data:
                break
            while len(data) < frame_bytes:
                chunk = process.stdout.read(frame_bytes - len(data))
                if not chunk:
                    break
                data += chunk
            if len(data) != frame_bytes:
                raise ValueError("FFmpeg returned a truncated video frame")
            yield yielded / probe_fps, np.frombuffer(data, dtype=np.uint8).reshape(size, size, 3).copy()
            yielded += 1
        return_code = process.wait()
        error = process.stderr.read().decode("utf-8", errors="replace").strip()
        if yielded == 0 or return_code != 0:
            raise ValueError(f"FFmpeg could not decode video: {error or 'no frames returned'}")
    finally:
        if process.poll() is None:
            process.kill()
        process.stdout.close()
        process.stderr.close()
```

- [ ] **Step 4: Extract the OpenCV probe generator**

Move OpenCV-specific opening, seeking, and reading into `_iter_opencv_frames`. It must raise when `VideoCapture` is closed or the first `read()` fails, and must always call `release()`.

```python
def _iter_opencv_frames(video_path: str, probe_fps: float, max_duration: float):
    cap = cv2.VideoCapture(video_path)
    if not cap.isOpened():
        cap.release()
        raise ValueError("OpenCV could not open the video")
    try:
        fps = cap.get(cv2.CAP_PROP_FPS)
        if not fps or fps != fps:
            fps = 25.0
        sec = 0.0
        yielded = 0
        while sec <= max_duration:
            cap.set(cv2.CAP_PROP_POS_FRAMES, int(fps * sec))
            ok, frame = cap.read()
            if not ok:
                if yielded == 0:
                    raise ValueError("OpenCV could not decode the first frame")
                break
            yielded += 1
            yield sec, cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            sec += 1.0 / probe_fps
    finally:
        cap.release()
```

- [ ] **Step 5: Feed either decoder into the unchanged selection rules**

In `extract_video_frames`, request the first OpenCV frame before yielding anything. On failure, switch to FFmpeg. Apply the existing scene threshold, duplicate threshold, base sampling interval, maximum duration, and maximum selected-frame count to that raw iterator.

```python
probe_fps = extract_fps * 4.0
try:
    opencv_frames = _iter_opencv_frames(video_path, probe_fps, max_duration)
    first = next(opencv_frames)
    raw_frames = chain([first], opencv_frames)
except (StopIteration, ValueError) as opencv_error:
    print(f"⚠️ OpenCV decode failed, using FFmpeg fallback: {opencv_error}")
    raw_frames = _iter_ffmpeg_frames(video_path, probe_fps, max_duration)
```

The remainder of the generator consumes `(sec, frame_rgb)` and preserves the existing decision rules verbatim.

- [ ] **Step 6: Run the forced-fallback test**

Run:

```powershell
.\.venv\Scripts\python.exe -m unittest tests.test_video_extractor.VideoDecodeTests.test_ffmpeg_fallback_when_opencv_cannot_open -v
```

Expected: PASS with at least one RGB frame.

- [ ] **Step 7: Commit**

```powershell
git add src/inference-worker/media/video_extractor.py src/inference-worker/tests/test_video_extractor.py
git commit -m "fix: fall back to ffmpeg for video decoding"
```

### Task 3: Test real MKV, AVI, MP4, and MOV containers

**Files:**
- Modify: `src/inference-worker/tests/test_video_extractor.py`
- Create at test runtime only: `%TEMP%/framescout-video-tests/*.{mkv,avi,mp4,mov}`

**Interfaces:**
- Consumes: system FFmpeg during development tests and public `extract_video_frames`.
- Produces: integration proof for all four required containers.

- [ ] **Step 1: Generate deterministic fixtures in `setUpClass`**

Use `tempfile.TemporaryDirectory()` and these codec/container pairs:

```python
commands = {
    "mkv": ["-c:v", "libx264"],
    "avi": ["-c:v", "mpeg4"],
    "mp4": ["-c:v", "libx264", "-pix_fmt", "yuv420p"],
    "mov": ["-c:v", "libx264", "-pix_fmt", "yuv420p"],
}
```

Each command begins with:

```python
[ffmpeg, "-y", "-f", "lavfi", "-i", "testsrc2=size=320x180:rate=12", "-t", "2"]
```

- [ ] **Step 2: Add one subtest per container**

```python
def test_extracts_required_containers(self):
    for extension, path in self.samples.items():
        with self.subTest(extension=extension):
            frames = list(extract_video_frames(str(path), max_duration=2, max_frames=4))
            self.assertGreater(len(frames), 0)
            self.assertTrue(all(frame.ndim == 3 and frame.shape[2] == 3 for _, frame in frames))
```

- [ ] **Step 3: Run all worker video tests**

Run:

```powershell
.\.venv\Scripts\python.exe -m unittest tests.test_video_extractor -v
```

Expected: MKV, AVI, MP4, MOV, extension, and forced-fallback tests all pass.

- [ ] **Step 4: Commit**

```powershell
git add src/inference-worker/tests/test_video_extractor.py
git commit -m "test: verify mkv avi mp4 and mov extraction"
```

### Task 4: Package FFmpeg beside the Nuitka worker

**Files:**
- Modify: `src/inference-worker/build_nuitka.ps1`
- Modify: `src/inference-worker/BUILD.md`

**Interfaces:**
- Consumes: `ffmpeg.exe` resolved by `Get-Command ffmpeg` at build time.
- Produces: `src/FrameScout-UI/src-tauri/bin/ai_worker/ffmpeg.exe` and final `ai_worker/ffmpeg.exe`.

- [ ] **Step 1: Add build-time FFmpeg validation**

```powershell
$ffmpegCommand = Get-Command ffmpeg -ErrorAction SilentlyContinue
if (-not $ffmpegCommand) {
    throw "FFmpeg not found. Install ffmpeg and ensure ffmpeg.exe is on PATH before packaging."
}
$ffmpegSource = $ffmpegCommand.Source
```

- [ ] **Step 2: Copy FFmpeg to the worker output**

After locating `$exeDir`:

```powershell
Copy-Item -Force $ffmpegSource (Join-Path $exeDir "ffmpeg.exe")
```

For one-file deployment, add:

```powershell
Copy-Item -Force (Join-Path $exeDir "ffmpeg.exe") $tauriBin
```

The onedir deployment already copies every item in `$exeDir`.

- [ ] **Step 3: Document the packaged fallback**

Update `BUILD.md` to state that FFmpeg is mandatory at build time but not required on the end user's PATH because it is copied beside the worker.

- [ ] **Step 4: Validate the PowerShell script syntax**

Run:

```powershell
$tokens=$null; $errors=$null; [void][System.Management.Automation.Language.Parser]::ParseFile(
  (Resolve-Path '.\build_nuitka.ps1'), [ref]$tokens, [ref]$errors
); if($errors){$errors | Format-List; exit 1}
```

Expected: exit code 0 and no parser errors.

- [ ] **Step 5: Commit**

```powershell
git add src/inference-worker/build_nuitka.ps1 src/inference-worker/BUILD.md
git commit -m "build: package ffmpeg with inference worker"
```

### Task 5: Build and validate the standalone Windows application

**Files:**
- Modify binary artifacts only: `src/FrameScout-UI/src-tauri/bin/ai_worker/`, `dist/FrameScout-3.2.0-windows-x64/`
- Update: `dist/FrameScout-3.2.0-windows-x64/BUILD-REPORT.md`

**Interfaces:**
- Consumes: tested source, models, FFmpeg, Nuitka, npm, Rust, Tauri.
- Produces: standalone Windows 11 folder with working video indexing.

- [ ] **Step 1: Run source validations**

```powershell
cd src\inference-worker
.\.venv\Scripts\python.exe -m unittest tests.test_video_extractor -v
cd ..\FrameScout-UI\src-tauri
cargo test --locked -q
cd ..
npm run type-check
```

Expected: all tests pass.

- [ ] **Step 2: Rebuild and deploy the Nuitka worker**

```powershell
cd ..\inference-worker
.\build_nuitka.ps1 -NoTorch
```

Expected: `bin/ai_worker/ai_worker.exe`, `bin/ai_worker/ffmpeg.exe`, and both model directories exist.

- [ ] **Step 3: Build Tauri production**

```powershell
cd ..\FrameScout-UI
npm run tauri build
```

Expected: `src-tauri/target/release/framescout-ui.exe`.

- [ ] **Step 4: Assemble the standalone folder without touching user data**

Copy the new Tauri executable and complete `ai_worker` directory over the build output only. Do not access or modify `%LOCALAPPDATA%/FrameScout-Offline_AI_Search-Global/*.db`.

- [ ] **Step 5: End-to-end smoke test**

Launch `FrameScout.exe`, verify the worker reaches port 5555, index the generated MKV and AVI through the worker protocol, and assert at least one `FrameResult` per file. Close the app normally and assert no `ai_worker.exe` remains.

- [ ] **Step 6: Update build report and hashes**

Record the successful MKV/AVI/MP4/MOV tests, FFmpeg fallback test, production executable SHA-256, dependencies, and any build errors with their fixes.
