# FrameScout Inference Worker — 构建与打包手册

> 面向 **Windows（PowerShell）** 开发环境，按「环境 → Protobuf 生成 → 开发运行 → PyInstaller 打包 → 部署到 Tauri」组织。
> 约定：`$ROOT` = `src/inference-worker`（本目录）。所有命令默认在该目录下执行，虚拟环境位于 `.venv`。

---

## 0. 目录约定

```text
FrameScout_Global/
├── proto\                     # .proto 与 protoc 编译器（protoc.exe）
│   ├── search.proto
│   └── protoc.exe
└── src\inference-worker\       # 本手册的工作目录（$ROOT）
    ├── main.py
    ├── server.py
    ├── config.py
    ├── search_pb2.py           # 由 search.proto 生成（步骤 2）
    ├── models\                 # SigLIP2 + EasyOCR 模型（运行时需要）
    ├── .venv\
    └── requirements.txt
```

> 若你的 `proto` 实际位置不同，请统一替换下方 `--proto_path`（简写 `-I`）与 `search.proto` 的相对路径。

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

`requirements.txt` 已覆盖：`onnxruntime`（`-directml` / `-gpu`）、`transformers`、`Pillow`、`numpy`、`easyocr`、`opencv-python-headless`、`pyzmq`、`protobuf`。

> 旧 `Notes.txt` 里散落的逐条 `pip install` 已统一进 `requirements.txt`，无需手动安装。

---

## 2. 生成 Protobuf 代码（仅当 `.proto` 变动时）

`search.proto` → `search_pb2.py`，放到 `$ROOT`（与 `main.py` 同级）。

**方式 A — 仓库自带 `protoc.exe`（推荐，离线稳定）：**

```powershell
..\proto\protoc.exe --proto_path=..\proto --python_out=. ..\proto\search.proto
```

**方式 B — `grpcio-tools`（跨平台）：**

```powershell
python -m grpc_tools.protoc --proto_path=..\proto --python_out=. ..\proto\search.proto
```

两条命令等价，产出同一个 `search_pb2.py`。校验：

```powershell
python -c "import search_pb2; print('search_pb2 OK')"
```

---

## 3. 开发期运行

```powershell
(.venv) python main.py
```

预期：打印日志文件路径 → 加载 processor → 初始化 Vision/Text ONNX → `🚀 AI Worker online! Listening on port 5555...`

冒烟测试（另开终端，发 `PING_ENGINE`）：

```powershell
python -c "import zmq, search_pb2; c=zmq.Context().socket(zmq.REQ); c.connect('tcp://127.0.0.1:5555'); r=search_pb2.EncodeRequest(); r.text='PING_ENGINE'; c.send(r.SerializeToString()); print(search_pb2.EncodeResponse().ParseFromString(c.recv()))"
```

---

## 4. PyInstaller 打包

> 本项目已迁移到 **ONNX Runtime**，不依赖 `torch` 做推理，故无需 `--hidden-import "torch"`。默认按「无 torch」打包。

### 4.1 调试版（带控制台窗口，推荐先跑通）

```powershell
(.venv) pyinstaller --noconfirm --onedir --console --name "ai_worker" `
    --hidden-import "transformers" `
    --hidden-import "easyocr" `
    --hidden-import "onnxruntime" `
    --hidden-import "cv2" `
    --collect-all "easyocr" `
    main.py
```

### 4.2 发布版（无控制台，供 Tauri 后台启动）

在调试版基础上加 `--noconsole`，并**显式收集业务子模块**（否则 `media/engines/utils` 可能漏打）：

```powershell
(.venv) pyinstaller --noconfirm --onedir --noconsole --name "ai_worker" `
    --hidden-import "transformers" `
    --hidden-import "easyocr" `
    --hidden-import "onnxruntime" `
    --hidden-import "cv2" `
    --hidden-import "media" `
    --hidden-import "engines" `
    --hidden-import "utils" `
    --collect-all "easyocr" `
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

### 4.3 补齐运行时资源

PyInstaller 不会自动打包模型与 `transformers` 处理器，需手动放置或让代码自动下载：

- `models\siglip2-base\` → 放进 `dist\ai_worker\models\siglip2-base\`
- `models\easyocr\`     → 放进 `dist\ai_worker\models\easyocr\`

（也可通过 `config.py` 的路径解析从外部指定模型目录。）

### 4.4 部署到 Tauri

```powershell
# ⚠️ 本命令在 $ROOT（src\inference-worker）下执行，下方路径均为“相对于 $ROOT”的写法。
# （若从仓库根目录执行，源/目标应改为 "src\inference-worker\dist\ai_worker" 与 "src\FrameScout-UI\src-tauri\bin\ai_worker"。）

# 删旧
Remove-Item -Recurse -Force "..\FrameScout-UI\src-tauri\bin\ai_worker"

# 复制整个新目录（含 _internal/ 运行时与 models/ 模型）
# 请确保 §4.3 已先把模型放进 dist\ai_worker\models，否则 _internal 会被拷过去而模型不会。
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
🚀 AI Worker online! Listening on port 5555...
```

---

## 5. 验证与排错

- **日志路径**：`%TEMP%\FrameScout-Offline_AI_Search-Global\logs\ai_worker_*.log`
- **端口监听**：`Test-NetConnection -ComputerName 127.0.0.1 -Port 5555`（`TcpTestSucceeded: True` 即通过）
- **进程内存**：`ai_worker.exe` 内存稳定在数百 MB 以上说明模型已加载完成
- **前端 404**：通常是 Vite 已启动但 Tauri 主循环在等 AI Worker；Worker 正常后若仍 404，检查 `tauri.conf.json` 的 `devUrl` 与 Vite 端口一致（默认 `http://localhost:1421`）
- **无害 WARNING**：`Failed to collect submodules for 'onnxruntime.quantization'`（缺 `onnx`）、`Library nvcuda.dll ... not found`（无 NVIDIA GPU，用 DirectML，正常）

---

## 6. 常用命令速查（Cheat Sheet）

| 目的 | 命令 |
| --- | --- |
| 激活环境 | `.\.venv\Scripts\Activate.ps1` |
| 安装依赖 | `pip install -r requirements.txt` |
| 生成 protobuf | `..\proto\protoc.exe --proto_path=..\proto --python_out=. ..\proto\search.proto` |
| 导入自检 | `python -c "import config, server, search_pb2; print('OK')"` |
| 开发运行 | `python main.py` |
| 调试打包（console） | 见 §4.1 |
| 正式打包（noconsole） | 见 §4.2 |

---

## 7. 依赖拆分建议

运行时（`requirements.txt`，已有）与开发/打包期（`requirements-dev.txt`）可分开：

```text
# requirements-dev.txt
pyinstaller>=6.0.0
grpcio-tools>=1.60.0
onnx>=1.15.0
onnxscript>=0.1.0
```

安装：`pip install -r requirements-dev.txt`
