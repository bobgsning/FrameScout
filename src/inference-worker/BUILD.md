# FrameScout Inference Worker — 构建与打包手册

> 面向 **Windows（PowerShell）** 开发环境，按「环境 → Protobuf 生成 → 开发运行 → Nuitka 打包 → 部署到 Tauri」组织。
> 约定：`$ROOT` = `src/inference-worker`（本目录）。所有命令默认在该目录下执行，虚拟环境位于 `.venv`。

---

## 0. 目录约定

```text
FrameScout_Global/
├── proto\                     # .proto 与 protoc 编译器（protoc.exe）
│   ├── framescout.proto
│   └── protoc.exe
└── src\inference-worker\       # 本手册的工作目录（$ROOT）
    ├── main.py
    ├── server.py
    ├── config.py
    ├── framescout_pb2.py           # 由 framescout.proto 生成（步骤 2）
    ├── models\                 # SigLIP2 + BGE-M3 模型（OCR 默认权重随 wheel 自带，无需下载）
    ├── .venv\
    └── requirements.txt
```

> 若你的 `proto` 实际位置不同，请统一替换下方 `--proto_path`（简写 `-I`）与 `framescout.proto` 的相对路径。

---

## 1. 环境搭建（首次一次性操作）

### 1.1 创建并激活虚拟环境

```powershell
cd src\inference-worker
python -m venv .venv

# 若脚本执行被禁止，先在本进程放宽策略（仅当前窗口有效）
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope Process

.\.venv\Scripts\Activate.ps1   # 激活后提示符前出现 (.venv)
```

### 1.2 安装依赖

```powershell
(.venv) pip install --upgrade pip
(.venv) pip install -r requirements.txt
```

`requirements.txt` 已覆盖：`onnxruntime-directml`、`transformers`、`Pillow`、`numpy`、`rapidocr_onnxruntime`、`opencv-python-headless`、`pyzmq`、`protobuf`。

> **⚠️ 安装顺序陷阱（关键）**：`rapidocr_onnxruntime` 的依赖声明是标准包名
> `onnxruntime>=1.7.0`，而 pip 不会把 `onnxruntime-directml` 视为它的替代，
> 所以直接 `pip install -r requirements.txt` 会**再拉一个标准 onnxruntime**，
> 与 directml 同名冲突，最终 import 到的可能是不含 `DmlExecutionProvider` 的标准包
> —— RapidOCR 会静默退回 CPU（功能正常、只是慢）。
>
> 正确顺序：
> ```powershell
> (.venv) pip install onnxruntime-directml>=1.24.4
> (.venv) pip install --no-deps rapidocr_onnxruntime
> # 其余依赖（transformers / Pillow / numpy / opencv / pyzmq / protobuf 等）
> # 若未装齐再补：pip install -r requirements.txt
> ```
> 验证（应看到 `DmlExecutionProvider`）：
> ```powershell
> (.venv) python -c "import onnxruntime as ort; print(ort.get_available_providers())"
> ```

> 旧 `Notes.txt` 里散落的逐条 `pip install` 已统一进 `requirements.txt`，无需手动安装。

---

## 2. 生成 Protobuf 代码（仅当 `.proto` 变动时）

`framescout.proto` → `framescout_pb2.py`，放到 `$ROOT`（与 `main.py` 同级）。

**方式 A — 仓库自带 `protoc.exe`（推荐，离线稳定）：**

```powershell
..\proto\protoc.exe --proto_path=..\proto --python_out=. ..\proto\framescout.proto
```

**方式 B — `grpcio-tools`（跨平台）：**

```powershell
python -m grpc_tools.protoc --proto_path=..\proto --python_out=. ..\proto\framescout.proto
```

两条命令等价，产出同一个 `framescout_pb2.py`。校验：

```powershell
python -c "import framescout_pb2; print('framescout_pb2 OK')"
```

---

## 3. 开发期运行

```powershell
(.venv) python main.py
```

预期：打印日志文件路径 → 加载 processor → 初始化 Vision/Text ONNX → `🚀 AI Worker online! Listening on port 16666...`

冒烟测试（另开终端，发 `PING_ENGINE`）：

```powershell
python -c "import zmq, framescout_pb2; c=zmq.Context().socket(zmq.REQ); c.connect('tcp://127.0.0.1:16666'); r=framescout_pb2.EncodeRequest(); r.text='PING_ENGINE'; c.send(r.SerializeToString()); print(framescout_pb2.EncodeResponse().ParseFromString(c.recv()))"
```

---

## 4. Nuitka 打包（当前首选）

> Nuitka 把 Python 逐行翻译成 C++ 再经 MSVC 编译成原生二进制，**零业务代码改动**即可
> 消灭 PyInstaller 的 Temp 解压延时（冷启动毫秒级）、体积虚胖（瘦 30~50%）与杀软误报。
> 脚本已封装好全部细节，一键运行：

```powershell
# 发布版（standalone + onefile，单文件、体积最小、编译最慢）
.\build_nuitka.ps1

# 调试版（仅 standalone，编译快、便于排错）
.\build_nuitka.ps1 -Onedir

# 只编译、不部署到 Tauri bin
.\build_nuitka.ps1 -SkipDeploy
```

### 4.1 前置条件

1. 已建好 `.venv` 并装齐依赖，且 `onnxruntime-directml` 自检通过（见 §1.2）；
2. 已安装 Nuitka：`pip install nuitka zstandard`（`zstandard` 供 onefile 压缩）；
3. 已安装 **Visual Studio Build Tools**（工作负载「使用 C++ 的桌面开发」）。
   脚本用 `--msvc=latest`，会通过 vswhere 自动定位 MSVC（MSVC 对 DirectML/Win32
   兼容性远高于 MinGW，务必不要用 MinGW）。

### 4.2 脚本替你处理的三个坑（对应战略规划书 7.3）

| 坑 | 对策（脚本已内建） |
| --- | --- |
| ONNX GPU Provider 动态库丢失 | `--include-data-dir=…\onnxruntime\capi=onnxruntime/capi`，把 `onnxruntime.dll` / `onnxruntime_providers_shared.dll` / `DirectML.dll` 整目录打入 |
| 编译器冲突 | `--msvc=latest` |
| 模型资源不随 exe 分发 | 编译后把整个 `models\`（siglip2-base + bge-m3）复制到 exe 同级目录 |

> **ZMQ 句柄残留**是运行期问题（打包不涉及）：已要求 `server.py` 侧设 `LINGER=0`，
> 且 Tauri 退出钩子里会 kill worker。此点与打包脚本无关，但验收时需一并确认无野进程。

### 4.3 部署到 Tauri

脚本默认在编译后自动把产物复制到 `..\FrameScout-UI\src-tauri\bin\ai_worker\`。
Tauri 侧 `worker_process.rs` 按 `bin\ai_worker\ai_worker.exe` 定位，两种模式都兼容：
- **onefile**：`bin\ai_worker\ai_worker.exe` + `bin\ai_worker\models\`（siglip2-base + bge-m3）
- **onedir**：`bin\ai_worker\ai_worker.exe` + 依赖文件 + `models\`（siglip2-base + bge-m3）

随后回前端目录 `npm run tauri dev`（或 `npm run dev:pro`）启动即可。

---

## 5. PyInstaller 打包（legacy 备选）

> 本项目已迁移到 **ONNX Runtime**，不依赖 `torch` 做推理，故无需 `--hidden-import "torch"`。默认按「无 torch」打包。
> Nuitka（§4）为首选；本节保留给需要 PyInstaller 的场景（如 Nuitka 编译环境暂缺）。

### 5.1 调试版（带控制台窗口，推荐先跑通）

```powershell
(.venv) pyinstaller --noconfirm --onedir --console --name "ai_worker" `
    --hidden-import "transformers" `
    --hidden-import "rapidocr_onnxruntime" `
    --hidden-import "onnxruntime" `
    --hidden-import "cv2" `
    --collect-all "rapidocr_onnxruntime" `
    main.py
```

### 5.2 发布版（无控制台，供 Tauri 后台启动）

在调试版基础上加 `--noconsole`，并**显式收集业务子模块**（否则 `media/engines/utils` 可能漏打）：

```powershell
(.venv) pyinstaller --noconfirm --onedir --noconsole --name "ai_worker" `
    --hidden-import "transformers" `
    --hidden-import "rapidocr_onnxruntime" `
    --hidden-import "onnxruntime" `
    --hidden-import "cv2" `
    --hidden-import "media" `
    --hidden-import "engines" `
    --hidden-import "utils" `
    --collect-all "rapidocr_onnxruntime" `
    --collect-submodules "media" `
    --collect-submodules "engines" `
    --collect-submodules "utils" `
    main.py
```

若仍报 `onnxruntime` 的 DLL 缺失（`--collect-all` 未自动包含时），用 `--add-binary` 显式打入：

```powershell
# 先确认 DLL 实际文件名
Get-ChildItem "$env:VIRTUAL_ENV\Lib\site-packages\onnxruntime\capi\*.dll"

# 再在打包命令中加入（以你目录里的文件名为准，通常含下列几个）：
--add-binary "$env:VIRTUAL_ENV\Lib\site-packages\onnxruntime\capi\onnxruntime.dll;onnxruntime\capi" `
--add-binary "$env:VIRTUAL_ENV\Lib\site-packages\onnxruntime\capi\onnxruntime_providers_shared.dll;onnxruntime\capi"
```

### 5.3 补齐运行时资源

PyInstaller 不会自动打包模型与 `transformers` 处理器，需手动放置或让代码自动下载：

- `models\siglip2-base\` → 放进 `dist\ai_worker\models\siglip2-base\`
- `models\ocr\`          → （占位）默认 PP-OCRv4 权重随 wheel 自带，无文件需搬运

（也可通过 `config.py` 的路径解析从外部指定模型目录。）

### 5.4 部署到 Tauri

```powershell
# ⚠️ 本命令在 $ROOT（src\inference-worker）下执行，下方路径均为“相对于 $ROOT”的写法。
# （若从仓库根目录执行，源/目标应改为 "src\inference-worker\dist\ai_worker" 与 "src\FrameScout-UI\src-tauri\bin\ai_worker"。）

# 删旧
Remove-Item -Recurse -Force "..\FrameScout-UI\src-tauri\bin\ai_worker"

# 复制整个新目录（含 _internal/ 运行时与 models/ 模型）
# 请确保 §5.3 已先把模型放进 dist\ai_worker\models，否则 _internal 会被拷过去而模型不会。
Copy-Item -Recurse -Force "dist\ai_worker" `
    "..\FrameScout-UI\src-tauri\bin\ai_worker"
```

然后回到前端目录启动：

```powershell
cd src\FrameScout-UI
npm run tauri dev        # 或 npm run dev:pro
```

预期看到：

```text
👻 Spawning AI Worker: "..."
💾 Connecting to local SQLite...
✅ Memory matrix loaded!
🚀 AI Worker online! Listening on port 16666...
```

---

## 6. 验证与排错

- **日志路径**：`%TEMP%\FrameScout-Offline_AI_Search-Global\logs\ai_worker_*.log`
- **端口监听**：`Test-NetConnection -ComputerName 127.0.0.1 -Port 16666`（`TcpTestSucceeded: True` 即通过）
- **进程内存**：`ai_worker.exe` 内存稳定在数百 MB 以上说明模型已加载完成
- **前端 404**：通常是 Vite 已启动但 Tauri 主循环在等 AI Worker；Worker 正常后若仍 404，检查 `tauri.conf.json` 的 `devUrl` 与 Vite 端口一致（默认 `http://localhost:1421`）
- **无害 WARNING**：`Failed to collect submodules for 'onnxruntime.quantization'`（缺 `onnx`）、`Library nvcuda.dll ... not found`（无 NVIDIA GPU，用 DirectML，正常）

---

## 7. 常用命令速查（Cheat Sheet）

| 目的 | 命令 |
| --- | --- |
| 激活环境 | `.\.venv\Scripts\Activate.ps1` |
| 安装依赖 | `pip install -r requirements.txt` |
| 生成 protobuf | `..\proto\protoc.exe --proto_path=..\proto --python_out=. ..\proto\framescout.proto` |
| 导入自检 | `python -c "import config, server, framescout_pb2; print('OK')"` |
| 开发运行 | `python main.py` |
| Nuitka 打包（首选） | `.\build_nuitka.ps1` |
| 调试打包（console，PyInstaller） | 见 §5.1 |
| 正式打包（noconsole，PyInstaller） | 见 §5.2 |

---

## 8. 依赖拆分建议

运行时（`requirements.txt`，已有）与开发/打包期（`requirements-dev.txt`）可分开：

```text
# requirements-dev.txt
nuitka>=2.0.0
zstandard>=0.22.0
pyinstaller>=6.0.0
grpcio-tools>=1.60.0
onnx>=1.15.0
onnxscript>=0.1.0
```

安装：`pip install -r requirements-dev.txt`
