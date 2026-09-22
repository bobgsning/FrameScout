# FrameScout Inference Worker

> Python 推理服务：为 FrameScout 的 Rust/Tauri 核心提供**离线、本地**的多模态特征提取能力。
> 通过 ZeroMQ + Protobuf（`src/proto/search.proto`）与核心通信，默认监听 `tcp://127.0.0.1:5555`。

---

## 1. 它做什么

| 能力 | 实现 | 说明 |
| --- | --- | --- |
| 语义向量（SigLIP 2） | `engines/siglip_engine.py` | 图像 / 文本统一编码为 **768 维**向量（ONNX Runtime 推理） |
| 文字识别（EasyOCR） | `engines/ocr_engine.py` | 离线多语言 OCR；**懒加载 + 多语言缓存**（`OCRManager`），按需构建 reader |
| 视频抽帧 | `media/video_extractor.py` | OpenCV 按设定帧率抽取关键帧（详见 §6） |
| 图片安全解码 | `media/image_io.py` | 兼容中文路径、`BGR→RGB` 校正 |

所有模型均在本地加载，**不依赖任何网络请求**。

---

## 2. 目录结构

```text
src/inference-worker/
├── main.py                 # 极简启动入口（仅做引导）
├── server.py               # ZeroMQ 消息分发与业务调度中心
├── config.py               # 全局配置、路径解析、日志目录
├── license_gen.py          # Pro License 签发工具（开发者本地使用，私钥勿提交）
├── requirements.txt        # 运行时依赖
├── ai_worker.spec          # PyInstaller 打包规格
├── engines/
│   ├── siglip_engine.py    # SigLIP 2（Vision & Text 封装）
│   └── ocr_engine.py       # OCRManager（EasyOCR 懒加载与多语言缓存）
├── utils/
│   ├── logger.py           # 日志重定向与日志文件初始化
│   ├── onnx_utils.py       # 硬件检测（CUDA / DirectML / CPU）与 ONNX 实例构建
│   └── vector_utils.py     # 维度安全保护（768D 归一化 / 整形）
├── media/
│   ├── image_io.py         # 图片安全解码
│   └── video_extractor.py  # 视频抽帧
└── models/                 # 由 scripts/download_models.py 生成（git 忽略）
    ├── siglip2-base/       # SigLIP 2 ONNX 模型（vision + text）
    └── easyocr/            # EasyOCR 模型与数据文件
```

---

## 3. 环境搭建

```powershell
cd src/inference-worker
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -r requirements.txt
```

`requirements.txt` 已覆盖：`onnxruntime`（`-directml` / `-gpu`）、`transformers`、`Pillow`、`numpy`、`easyocr`、`opencv-python-headless`、`pyzmq`、`protobuf`。

---

## 4. 生成 Protobuf 代码（仅当 `search.proto` 变更时）

`search.proto` → `search_pb2.py`（需放在本目录，与 `main.py` 同级）。

**方式 A — 仓库自带 `protoc.exe`（推荐，离线稳定）：**

```powershell
..\proto\protoc.exe --proto_path=..\proto --python_out=. ..\proto\search.proto
```

**方式 B — `grpcio-tools`（跨平台）：**

```powershell
python -m grpc_tools.protoc --proto_path=..\proto --python_out=. ..\proto\search.proto
```

校验：

```powershell
python -c "import search_pb2; print('search_pb2 OK')"
```

---

## 5. 开发运行

```powershell
python main.py
```

预期输出（节选）：

```text
🚀 AI Worker online! Listening on port 5555...
```

冒烟测试（另开终端，发送 `PING_ENGINE`）：

```powershell
python -c "import zmq, search_pb2; c=zmq.Context().socket(zmq.REQ); c.connect('tcp://127.0.0.1:5555'); r=search_pb2.EncodeRequest(); r.text='PING_ENGINE'; c.send(r.SerializeToString()); print(search_pb2.EncodeResponse().ParseFromString(c.recv()))"
```

---

## 6. 视频帧索引

> 索引一条视频时，worker 会：
> 1. 用 `media/video_extractor.py` 按设定帧率（默认 1 FPS，后续计划支持场景检测抽帧）抽取关键帧；
> 2. 对每一帧执行 SigLIP 2 向量编码与（可选）OCR；
> 3. 将每帧作为一个独立的 `FrameResult` 返回，携带 `timestamp`（相对视频起点的秒数）与同一 `file_path`；
> 4. Rust 核心据此建立索引，搜索命中后可**精确到秒**跳转到对应视频画面。
>
> **实现要点（单文件 / 批量两条链路均已打通抽帧）：**
> - `server.py` 的 `_handle_file`（以图搜图）与 `_handle_batch`（扫描/索引）都会用 `is_video_file`
>   判断：视频走 `extract_video_frames`，图片走 `load_image_pil`，统一产出带 `timestamp` 的帧结果。
> - 批量分支按 `MAX_FRAMES_PER_ENCODING_BATCH` 分块送入 ONNX，限定单次推理帧数，避免长视频撑爆显存。
> - 抽帧护栏：`MAX_VIDEO_DURATION`（秒）与 `MAX_FRAMES_PER_VIDEO`（帧）任一触发即停止，二者均见 `config.py`。
> - Rust 侧 `frame_vectors` 表主键为 `(path, timestamp)`，可容纳同一视频的多帧；
>   旧版 `path` 单主键库会在启动时自动迁移（见 `storage/db.rs::ensure_frame_vectors_schema`）。
> - 搜索时同一文件的多帧命中会收敛为得分最高的一帧，其 `timestamp` 用于秒级跳转
>   （见 `commands/search_cmd.rs` 去重逻辑的注释）。

---

## 7. 打包为独立可执行文件

正式发布时，worker 需用 PyInstaller 打包为 `ai_worker.exe`，再由 Tauri 启动。
完整步骤（含 DLL 处理、资源补齐、部署到 `src-tauri/bin/`）见 **[BUILD.md](./BUILD.md)**。

要点速记：

```powershell
# 调试版（带控制台，便于排错）
pyinstaller --noconfirm --onedir --console --name "ai_worker" `
    --hidden-import "transformers" --hidden-import "easyocr" `
    --hidden-import "onnxruntime" --hidden-import "cv2" `
    --collect-all "easyocr" main.py

# 发布版（无控制台）需额外显式收集业务子模块：
#   --hidden-import "media" --hidden-import "engines" --hidden-import "utils"
#   --collect-submodules "media" --collect-submodules "engines" --collect-submodules "utils"
```

---

## 8. 配置与路径

- **模型目录**：`config.py` 的 `get_model_dirs()` 返回 `models/siglip2-base` 与 `models/easyocr`；首次运行可用 `scripts/download_models.py` 下载（约 1.5GB）。
- **日志目录**：`%TEMP%/FrameScout-Offline_AI_Search-Global/logs/`。
- **离线环境变量**：`config.py` 设置 `USE_ONLINE=0` 等，强制 `transformers` 不联网、不写缓存。

---

## 9. 常见问题

| 现象 | 排查 |
| --- | --- |
| 启动报 `ImportError: No module named 'media'` | 打包时未收集子模块，加 `--collect-submodules "media"`（见 BUILD.md §4.2） |
| onnxruntime DLL 缺失 | 用 `--add-binary` 显式打入 `onnxruntime\capi\*.dll` |
| 端口 5555 未监听 | `Test-NetConnection 127.0.0.1 -Port 5555` 验证；检查 worker 是否真正 `online` |
| EasyOCR 首次很慢 | 首次构建 reader 并加载检测/识别模型属正常，后续命中缓存 |
