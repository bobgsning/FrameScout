# FrameScout Inference Worker

> Python 推理服务：为 FrameScout 的 Rust/Tauri 核心提供**离线、本地**的多模态特征提取能力。
> 通过 ZeroMQ + Protobuf（`src/proto/framescout.proto`）与核心通信，默认监听 `tcp://127.0.0.1:16666`。

---

## 1. 它做什么

| 能力 | 实现 | 说明 |
| --- | --- | --- |
| 语义向量（SigLIP 2） | `engines/siglip_engine.py` | 图像 / 文本统一编码为 **768 维**向量（ONNX Runtime 推理） |
| 文字识别（RapidOCR） | `engines/ocr_engine.py` | 离线多语言 OCR；`OCRManager` 按 rec 模型分组懒加载——4 内置（英/简中/繁中/日）+ 6 可下载（俄/韩/德/法/西 + 英文优化变体 en_opt）（详见 [OCR_MULTILANG.md](./OCR_MULTILANG.md)） |
| 文本向量（BGE-M3） | `engines/bge_engine.py` | OCR 文本编码为 **1024 维** dense（第二条检索公路，与视觉互补；模型缺省不阻断启动） |
| 视频抽帧 | `media/video_extractor.py` | OpenCV 按设定帧率抽取关键帧（详见 §6） |
| 图片安全解码 | `media/image_io.py` | 兼容中文路径、`BGR→RGB` 校正 |

所有模型均在本地加载，**不依赖任何网络请求**。

---

## 2. 目录结构

```text
src/inference-worker/
├── main.py                 # 极简启动入口（仅做引导）
├── server.py               # ZeroMQ 消息分发与业务调度中心
├── config.py               # 全局配置、路径解析、日志目录、OCR 语言目录读取
├── framescout_pb2.py       # 由 framescout.proto 生成（步骤 4，与 main.py 同级）
├── framescout_pb2.pyi      # 同名生成的类型存根
├── requirements.txt        # 运行时依赖
├── build_nuitka.ps1        # Nuitka 打包脚本（当前首选）
├── ocr_manifest.json       # OCR 语言目录单一事实源（语言码 / 分层 / 模型与字典 / 下载 URL / SHA256）
├── scripts/
│   └── benchmark.py        # 性能基准（索引耗时 + 搜索延迟 P50/P95/mean）
├── engines/
│   ├── siglip_engine.py    # SigLIP 2（Vision & Text 封装）
│   ├── ocr_engine.py       # OCRManager（RapidOCR 懒加载与多语言缓存）
│   └── bge_engine.py       # BGE-M3 dense 文本向量（第二条检索公路）
├── utils/
│   ├── logger.py           # 日志重定向与日志文件初始化
│   ├── onnx_utils.py       # 硬件检测（CUDA / DirectML / CPU）与 ONNX 实例构建
│   └── vector_utils.py     # 维度安全保护（768D 归一化 / 整形）
├── media/
│   ├── image_io.py         # 图片安全解码
│   └── video_extractor.py  # 视频抽帧（OpenCV 主路径 + FFmpeg 兜底，见 §6）
└── models/                 # 由 scripts/download_models.py 生成（git 忽略）
    ├── siglip2-base/       # SigLIP 2 ONNX 模型（vision + text）
    ├── ocr/                # OCR 多语言 rec 模型：<code>/<rec_model> + <rec_dict>（en/ch_sim 随 wheel 自带；ch_tra/ja/en_opt/ru/ko/de/fr/es 按需获取）
    └── bge-m3/             # BGE-M3 文本向量（tokenizer + bge_m3_dense.onnx）
```

---

## 3. 环境搭建

```powershell
cd src/inference-worker
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -r requirements.txt
```

`requirements.txt` 已覆盖：`onnxruntime`（`-directml` / `-gpu`）、`transformers`、`Pillow`、`numpy`、`rapidocr_onnxruntime`、`opencv-python-headless`、`pyzmq`、`protobuf`。

---

## 4. 生成 Protobuf 代码（仅当 `framescout.proto` 变更时）

`framescout.proto` → `framescout_pb2.py`（需放在本目录，与 `main.py` 同级）。

**方式 A — 仓库自带 `protoc.exe`（推荐，离线稳定）：**

```powershell
..\proto\protoc.exe --proto_path=..\proto --python_out=. --pyi_out=. ..\proto\framescout.proto
```

**方式 B — `grpcio-tools`（跨平台）：**

```powershell
python -m grpc_tools.protoc --proto_path=..\proto --python_out=. --pyi_out=. ..\proto\framescout.proto
```

两条命令等价，同时产出 `framescout_pb2.py` 与 `framescout_pb2.pyi`。校验：

```powershell
python -c "import framescout_pb2; print('framescout_pb2 OK')"
```

---

## 5. 开发运行

```powershell
python main.py
```

预期输出（节选）：

```text
🚀 AI Worker online! Listening on port 16666...
```

冒烟测试（另开终端，发送 `PING_ENGINE`）：

```powershell
python -c "import zmq, framescout_pb2; c=zmq.Context().socket(zmq.REQ); c.connect('tcp://127.0.0.1:16666'); r=framescout_pb2.EncodeRequest(); r.text='PING_ENGINE'; c.send(r.SerializeToString()); print(framescout_pb2.EncodeResponse().ParseFromString(c.recv()))"
```

---

## 6. 视频帧索引

> 索引一条视频时，worker 会：
> 1. 用 `media/video_extractor.py` 做**场景检测抽帧**（帧间差 > `SCENE_CHANGE_THRESHOLD` 保留关键帧，
>    差 < `SIMILAR_FRAME_SKIP_THRESHOLD` 的相邻相似帧去重，1 FPS 兜底）；
> 2. 对每一帧执行 SigLIP 2 向量编码与（可选）OCR；
> 3. 将每帧作为一个独立的 `FrameResult` 返回，携带 `timestamp`（相对视频起点的秒数）与同一 `file_path`；
> 4. Rust 核心据此建立索引，搜索命中后可**精确到秒**跳转到对应视频画面。
>
> **实现要点（流式 + 场景检测）：**
> - `extract_video_frames` 是**生成器**（`yield`），`server.py` 的 `_process_frame_sub_batch`
>   按 `MAX_FRAMES_PER_ENCODING_BATCH` 边抽帧边编码边组装，内存占用恒定——
>   旧实现把视频全部帧一次性堆进 list（1080p×3600 ≈ 22GB）会 OOM。
> - `_handle_file`（以图搜图）与 `_handle_batch`（扫描/索引）都会用 `is_video_file` 判断，
>   视频走流式生成器，图片走 `load_image_pil`。
> - 场景检测阈值见 `config.py`：`SCENE_CHANGE_THRESHOLD = 25.0`、`SIMILAR_FRAME_SKIP_THRESHOLD = 2.5`。
>   抽帧以 0.25s 探测间隔采样，帧间差超过 `SCENE_CHANGE_THRESHOLD` 即判定场景切换并保留关键帧，
>   其余按 1 FPS 兜底 + 相邻去重。
> - 抽帧护栏：`MAX_VIDEO_DURATION`（秒）与 `MAX_FRAMES_PER_VIDEO`（帧）任一触发即停止。
> - **双帧来源（OpenCV 主 + FFmpeg 兜底）**：`video_extractor.py` 已重构为「帧来源 + 帧选择」分离——
>   `_iter_opencv_frames`（主路径）打不开容器或首帧失败时，自动切到 `_iter_ffmpeg_frames`
>   （调用 worker 同级的 `ffmpeg.exe`，见 §7），两路汇入同一套场景检测 + 相邻去重 + 护栏。
>   FFmpeg 路径的 stderr 写临时文件防管道死锁，并有 30s 读超时 watchdog（读不满一帧即 kill，
>   避免卡死 worker 的单线程 REQ/REP）；固定输出 256×256。
> - 搜索时同一文件的多帧命中会收敛为得分最高的一帧，其 `timestamp` 用于秒级跳转。

---

## 6.5 性能基准

```powershell
python scripts/benchmark.py --library <测试库路径> --output benchmarks
```

测量索引耗时（按文件数 / 帧数）+ 三种搜索查询的延迟（P50 / P95 / mean），输出 `benchmarks/YYYY-MM-DD.json`。
需先启动 FrameScout App（拉起 `ai_worker`，监听 `tcp://127.0.0.1:16666`）；脚本直连 ZMQ 端口测索引与查询延迟。

---

## 7. 打包为独立可执行文件

正式发布时，worker 需用 **Nuitka** 打包为原生 `ai_worker.exe`，再由 Tauri 启动。
Nuitka 把 Python 编译成机器码，**无 Temp 解压延时、体积瘦 30~50%、启动毫秒级**，
已取代 PyInstaller 成为当前首选。完整步骤见 **[BUILD.md](./BUILD.md) §4**。

一键打包（脚本已封装好 DirectML `capi` DLL、RapidOCR 权重、模型资源复制与部署）：

```powershell
# 发布版（standalone + onefile，单文件、体积最小）
.\build_nuitka.ps1

# 调试版（仅 standalone，编译更快、便于排错）
.\build_nuitka.ps1 -Onedir -NoTorch
```

脚本还会自动下载 **BtbN LGPL 版 FFmpeg**（固定版本 + SHA256 校验 + `-buildconf` 断言无 GPL/nonfree
组件），复制 `ffmpeg.exe` 到 worker 同级，作为 OpenCV 打不开部分 MKV/AVI 编解码器时的兜底解码器，
并供前端「切片段」命令调用；完整许可证文本见仓库根目录 `COPYING.LGPLv2.1`。

> 旧 PyInstaller 打包（`pyinstaller ...`）仅作 **legacy 备选**，
> 见 BUILD.md §5。新开发一律用 Nuitka。

---

## 8. 配置与路径

- **模型目录**：`config.py` 的 `get_model_dirs()` 返回 `models/siglip2-base` 与 `models/ocr`；SigLIP 首次运行可用 `scripts/download_models.py` 下载。OCR 的英文/简体中文权重（`ch_PP-OCRv4_rec`，中英合并字符集）随 `rapidocr_onnxruntime` wheel 自带、**无需下载**；繁體中文/日文由 `scripts/download_ocr_models.py` 随包内置，英文优化变体（`en_opt`）与俄/韩/德/法/西在设置页按需下载。语言目录与下载源见 `ocr_manifest.json`（详见 [OCR_MULTILANG.md](./OCR_MULTILANG.md)）。
- **日志目录**：`%TEMP%/FrameScout-Offline_AI_Search-Global/logs/`。
- **离线环境变量**：`config.py` 设置 `USE_ONLINE=0` 等，强制 `transformers` 不联网、不写缓存。

---

## 9. 常见问题

| 现象 | 排查 |
| --- | --- |
| 启动报 `ImportError: No module named 'media'` | Nuitka 脚本已自动收集子模块；仅 PyInstaller（legacy）需加 `--collect-submodules "media"` |
| onnxruntime DLL 缺失 | Nuitka 脚本已 `--include-data-dir` 整个 `onnxruntime\capi`；仅 PyInstaller 需手动 `--add-binary` |
| 端口 16666 未监听 | `Test-NetConnection 127.0.0.1 -Port 16666` 验证；检查 worker 是否真正 `online` |
| RapidOCR 首次很慢 | 首次构建 reader 并加载检测/识别模型属正常，后续命中缓存 |
| 启动时 `DmlExecutionProvider ... 8007000E` 异常 | GPU 显存不足。`utils/onnx_utils.py` 会捕获该异常并**自动回退纯 CPU**（打印 `Active providers (CPU fallback)`），功能不受影响、仅降速 |
| 搜索一直加载 | 旧版 RPC 无限等待超时已修复：搜索类 RPC 分类超时 5s/10s，索引 RPC 按批内视频数动态追加预算（`index_rpc_timeout_ms`）；若仍卡，检查 worker 日志确认模型是否成功加载 |
