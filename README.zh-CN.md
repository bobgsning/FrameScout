# 🔍 FrameScout —— 离线 AI 搜索

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.zh-CN.md"><strong>中文</strong></a>
</p>

## *100% 私有、完全离线、多模态桌面搜索引擎*

**用自然语言、OCR 文字或视觉相似度搜索你本地的图片与视频——无云端、无遥测、无账号。免费且开源（社区版）。**

[![License: Apache 2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4)](https://www.microsoft.com/windows)
![Architecture](https://img.shields.io/badge/architecture-Rust%20%2B%20Python%20%2B%20Vue-ff69b4)
[![AI-Assisted](https://img.shields.io/badge/AI--assisted-development-brightgreen)](AI_POLICY.md)

FrameScout 是一个**完全离线、由 AI 驱动的视觉搜索引擎**，面向你本地的图片与视频。
输入 *"海边和朋友一起看的日落"*、粘贴一张参考图，或是按图片中的文字来搜索——结果在毫秒之间呈现。所有计算都在你的机器上完成，没有任何数据离开你的电脑。

⬇️ [下载最新版本](https://github.com/bobgsning/FrameScout/releases) ·
📖 [快速开始](#快速开始开发) ·
🤝 [参与贡献](CONTRIBUTING.md)

> 🆕 **v3.1.0 已发布！** Smart Folders（智能文件夹）现已在后端完全持久化，重启后不再丢失规则；可对选中文件按需执行 OCR 并自定义语言；纯向量文件夹以 `?` 徽标诚实显示未知计数。 [查看更新日志](CHANGELOG.md#3-1-0-2026-09-23)。

---

> **只想上手试试？** 从 [https://github.com/bobgsning/FrameScout/releases](https://github.com/bobgsning/FrameScout/releases) 下载最新的 Windows 可执行文件即可，无需安装。解压后直接运行。

---

## 🎯 问题所在

你的硬盘里散落着数百 GB 的截图、照片和视频片段。
现有的解决方案逼你在两个糟糕的选项之间二选一：

☁️ **把所有东西上传到云端** —— 从此永远失去隐私。
📁 **继续用本地文件管理器** —— 然后花几个小时在文件夹里翻找。

FrameScout 提供了**第三条路**：完全在本机上运行的 AI 搜索。不上传、不订阅、不妥协。

---

## 🛠️ 已知限制

我们信奉透明。以下是 FrameScout 目前**还做不到**的事：

| 限制 | 应对方案 / 未来计划 |
| ---------- | ------------------------ |
| 仅支持 Windows 10/11（暂不支持 macOS / Linux） | 未来计划提供跨平台构建 |
| 视频抽帧使用固定的 1 FPS | 计划改用基于场景检测（scene-detection）的抽帧 |
| FlatVector 搜索为 O(N·d) 复杂度 | 当 N > 50K 时迁移到 Faiss / Annoy |
| 无文件系统实时监控 | 目前需手动重新扫描；计划引入 inotify / watchdog |
| 无 LLM 生成的图片描述 | 刻意而为——那需要联网 |
| "显示全部"模式在文件数 > 1 万时可能卡顿 | 计划引入虚拟滚动（virtual scrolling） |
| 纯向量搜索的 Smart Folder 匹配计数显示 `?` | 未来：增加近似向量计数 |
| 按需 OCR 仅支持单文件 | 计划于后续版本支持多文件批量 OCR |

---

## ✨ 功能特性

| 功能 | 描述 |
| --------- | ------------- |
| 🔒 **100% 离线** | 无需联网。模型已随包内置。你的数据永不离开你的电脑。 |
| 💡 **语义搜索（ONNX + SigLIP 2）** | 输入文字，按"含义"而非仅文件名找到匹配的图片。 |
| 🖼️ **以图搜图** | 拖入一张参考图，在你的图库中找到视觉上相似的图片。 |
| 🔍 **OCR 文字搜索** | 从图片中提取并建立文字索引。比如搜索 *"三月的收据"* 就能立刻找到。支持对选中文件按需执行 OCR 并自定义语言。 |
| 📋 **显示全部模式** | 加载所有已索引的文件。新文件按入库时间排序，始终置顶显示。 |
| 🎬 **视频帧索引** | 自动从视频中抽取关键帧，并与静态图片一同建立索引。 |
| 🧩 **视觉聚类** | 一键发现相似图片分组（重复、近似重复、连拍）。 |
| 📁 **智能文件夹（Smart Folders）** | 把任意搜索保存为动态文件夹，新增文件时自动更新。规则存储在数据库中，应用重启后依然保留。 |
| 📝 **个人笔记** | 给任意图片附加 Markdown 笔记，笔记可被搜索，下次更易找到。 |

### 🎬 视频帧语义搜索与即时跳转

FrameScout 不止能找到视频——它还能精确定位**具体的时间点**：

- **语义匹配**：用自然语言搜索（例如 *"栈桥下的日落"* 或 *"屏幕上的代码截图"*）。
- **一键即时跳转**：点击任意搜索结果，直接跳转到视频中的那一秒（`01:23:45`）。

> **v3.0.2 新增**：新索引的文件按入库时间排序，因此浏览模式下最新添加的内容始终排在最前。点击"📋 显示全部"按钮可一次性查看全部文件。

---

## 📁 仓库结构（关键目录）

```text
FrameScout/
├── src/
│   ├── FrameScout-UI/               # Vue 3 + Tauri 桌面应用（Rust 核心）
│   ├── inference-worker/       # Python AI 推理引擎（ONNX + SigLIP 2 + EasyOCR）
│   │   └── models/
│   │       ├── siglip2-base/   # SigLIP 2 ONNX 模型（视觉 + 文本）
│   │       └── easyocr/        # EasyOCR 模型存储（离线）
│   ├── proto/                  # ZeroMQ 通信协议（Protobuf）
├── scripts/                    # 工具脚本（模型下载等）
├── README.md
├── CONTRIBUTING.md
└── LICENSE
```

完整的目录树见 [TREE.md](./TREE.md)。

---

## 🏗️ 架构

FrameScout 采用**三进程架构**，以获得最佳性能、安全性与模块化：

```text
┌─────────────────────────────────────────────────────────┐
│                     Vue 3 前端                           │
│         （搜索控制台、结果网格、聚类视图）               │
└──────────────────────┬──────────────────────────────────┘
                       │  Tauri IPC
                       ▼
┌─────────────────────────────────────────────────────────┐
│                   Rust / Tauri 核心                      │
│  ┌──────────┐  ┌──────────────┐  ┌────────────────┐     │
│  │ 文件 I/O │  │ SQLite +     │  │ FlatVector     │     │
│  │ WalkDir  │  │ Rusqlite     │  │ 矩阵 (768 维)  │     │
│  └──────────┘  └──────────────┘  └────────────────┘     │
└──────────────────────┬──────────────────────────────────┘
                       │  ZeroMQ + Protobuf（REQ/REP）
                       ▼
┌─────────────────────────────────────────────────────────┐
│                Python 推理工作进程                       │
│  ┌──────────────────────────────────────────────────┐   │
│  │ ONNX + SigLIP 2  （文本 + 图像嵌入，768 维）      │   │
│  └──────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────┐   │
│  │ EasyOCR（文字提取，离线）                         │   │
│  └──────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────┐   │
│  │ OpenCV（视频抽帧，1 FPS）                         │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### 为何采用这样的架构？

| 决策 | 理由 |
| -------- | ------ |
| **Rust + Tauri**（而非 Electron） | 原生 WebView，二进制体积约为 Electron 的 1/10，内存占用仅为很小一部分 |
| **Python 工作进程**（而非全 Rust） | SigLIP 2 与 EasyOCR 在 Python 生态中已非常成熟，无需重复造轮子 |
| **ONNX Runtime**（而非 PyTorch） | 通用 GPU 加速（NVIDIA CUDA、AMD DirectML、Intel、CPU 回退）；体积更小 |
| **ZeroMQ + Protobuf**（而非 HTTP） | 微秒级延迟、类型安全、可完全离线工作 |
| **FlatVector 矩阵**（而非 Faiss） | 当 N < 50,000 时，O(N·d) 暴力检索已足够快（约 40ms）且零依赖 |
| **SQLite + WAL 模式** | 久经考验、嵌入式、支持写入期间并发读取 |

### 数据流

**索引一个文件夹**：Rust 遍历所选目录，收集文件路径，并通过 ZMQ 分批发送给 Python 工作进程。Python 为每张图片或视频帧生成 SigLIP 2 图像嵌入（768 维）与 OCR 文字，然后返回结果。Rust 将一切存入 SQLite 与内存中的 FlatVector 矩阵，同时前端显示实时进度。

**按文字搜索**：前端将查询发送给 Rust，Rust 转发给 Python 生成 SigLIP 2 文本嵌入。随后 Rust 用点积相似度在 FlatVector 矩阵中检索，将结果与 OCR、笔记、文件名得分合并，并把排序后的结果返回前端。

**按图片搜索**：流程与文字搜索一致，只是 Python 从参考图生成 SigLIP 2 图像嵌入，而非文本嵌入。

---

## 🔬 关键技术亮点

### 混合评分模型

FrameScout 将**四个信号**融合为一个统一的相关性得分：

```text
final_score = vector_similarity × 1.0      （语义含义）
            + ocr_match         × 2.0      （图片中的文字）
            + note_match        × 2.5      （用户标注）
            + filename_match    × 3.0      （路径 / 文件名匹配）
```

每个分量都可在界面中独立开关。结果实时评分、排序并分页。

### FlatVector 矩阵搜索

所有 768 维的 SigLIP 2 嵌入都存放在单个 `Vec<f32>` 中，以获得缓存友好的顺序访问：

```rust
pub struct FlatVectorMatrix {
    pub dim: usize,            // 768
    pub flat_vectors: Vec<f32>, // [v0_0..v0_767, v1_0..v1_767, ...]
    pub metadata: Vec<ImageMeta>,
}
```

搜索只是在连续内存上做一次简单的点积循环——**无 malloc、无间接寻址、无外部库**。在 50,000 张图片规模下，全量扫描约 40ms 完成。

---

## 📊 性能

测试环境：AMD Ryzen 7 5800X + 32GB 内存 + NVIDIA RTX 3070（ONNX + CUDA）。

| 集合规模 | 文字查询延迟 | 图片查询延迟 | 索引吞吐 |
| --------------- | ------------------ | ------------------- | -----------------|
| 1,000 张图片    | 0.8 ms             | 1.1 ms              | ~1,200 张/分钟   |
| 10,000 张图片   | 7.2 ms             | 9.5 ms              | ~800 张/分钟     |
| 50,000 张图片   | 38.0 ms            | 45.0 ms             | ~600 张/分钟     |

> **免责声明**：以上为开发环境的基准测试结果。实际性能取决于你的硬件与数据集特征。可使用随附的基准脚本在你的机器上复现（即将上线）。v3.0.2 引入了按索引时间排序，不影响查询延迟。

---

## 🚀 快速开始（开发）

### 先决条件

- Rust 1.75+（2021 edition）
- Python 3.10+
- Node.js 18+ 与 npm
- SigLIP 2 ONNX 模型文件（见下文）

### 构建与运行

```bash
# 克隆仓库
git clone https://github.com/bobgsning/FrameScout.git
cd FrameScout

# 1. 安装前端依赖
cd src/FrameScout-UI
npm install

# 2. 配置 Python 推理工作进程
cd ../inference-worker
python -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate
pip install -r requirements.txt

# > ⚠️ 此步骤需要联网。完成后 FrameScout 即可完全离线运行。
# 3. 下载 SigLIP 2 + EasyOCR 模型（一次性，总计约 1.5GB）（在 src/inference-worker 下执行）
python ../../scripts/download_models.py

# 4. 构建并运行（社区版）
# 回到前端目录进行构建
cd ../FrameScout-UI
npm run tauri dev          # 开发模式
# npm run tauri build      # 生产构建
```

### 打包推理工作进程

Python 推理工作进程必须被编译为独立可执行文件，供 Tauri 启动。

```bash
# 在仓库根目录执行
cd src/inference-worker
# 先激活你的虚拟环境
pyinstaller --noconfirm --onedir --console --name "ai_worker" \
    --hidden-import "transformers" \
    --hidden-import "easyocr" \
    main.py
```

构建成功后，将输出复制到 Tauri 二进制目录：

```bash
cp -r dist/ai_worker ../FrameScout-UI/src-tauri/bin/
```

之后 `npm run tauri dev` 便会自动启动已编译的工作进程。

> **注意**：打包后的工作进程会在自身所在位置相对的 `./models/` 目录中查找模型。复制到 `src/FrameScout-UI/src-tauri/bin/` 后，请确保该处存在 `models` 文件夹，或在 `main.py` 中调整路径。

### 首次启动

1. 出现启动画面：*"FRAME SCOUT NEURAL LINK ESTABLISHING..."*
2. SigLIP 2 + EasyOCR 模型载入内存（耗时因硬件而异）
3. 你将看到带搜索控制台的主界面
4. 点击 **Browse（浏览）** → 选择一个包含图片的文件夹
5. 点击 **Start Indexing（开始索引）** → 观看实时抽取进度
6. 在搜索栏输入描述 → 毫秒级获得结果

---

## 🗺️ 路线图

### v3.1.x（当前 —— 稳定性与打磨）

- [x] 核心搜索（文字 + 图片 + OCR + 笔记）
- [x] 视觉聚类
- [x] 智能文件夹
- [x] SigLIP 2 迁移（768 维嵌入，ONNX Runtime）
- [x] 按索引时间排序（新文件始终置顶）
- [x] "显示全部"模式（无分页的扁平列表）
- [x] RwLock 优化（扫描期间搜索不阻塞）
- [ ] 增强智能文件夹及其他用户体验
- [ ] 悬停显示文件元数据的提示框
- [ ] 单图删除（而非仅批量"幽灵清除"）
- [ ] 视频超时缩短至 10 秒 + 加载动画

### v3.1（跨平台与体验）

- [ ] 文件夹树侧边栏视图
- [ ] 多种视图模式（网格 / 列表 / 时间线）
- [ ] 导出 / 导入数据库（SQLite 备份）
- [ ] 数据时间线（可视化你的索引历史）

### v3.2（性能与智能）

- [ ] 当 N > 50K 时集成 Faiss
- [ ] 视频关键帧的场景检测
- [ ] 批量操作（多选删除 / 导出）
- [ ] 多模态融合搜索（文字 + 图片同时进行）
- [ ] macOS 构建（Apple Silicon 原生）
- [ ] Linux 构建（AppImage + Flatpak）

---

## 🤝 参与贡献

FrameScout 基于 Apache 2.0 开源。核心架构、算法与通信协议完全开放，可供研究与修改。

贡献指南见 [CONTRIBUTING.md](CONTRIBUTING.md)。

### 开源内容

| 组件 | 许可证 | 说明 |
| --------- | ------- | ----- |
| Rust / Tauri 核心（`src/FrameScout-UI/src-tauri/`） | Apache 2.0 | 完整源码开放 |
| Vue 3 前端（`src/FrameScout-UI/`） | Apache 2.0 | 完整源码开放 |
| Protobuf 定义（`src/proto/`） | Apache 2.0 | 完整源码开放 |
| Python 推理工作进程 | Apache 2.0 | 已包含在本仓库中 |

---

## 📄 许可证

FrameScout 核心基于 **Apache License 2.0** 授权。详见 [LICENSE](LICENSE)。

---

## ™️ 商标

"FrameScout"、"FrameScout —— Offline AI Search" 以及 FrameScout 徽标均为 Bob G. S. Ning 的商标。

其他所有商标归各自所有者所有。

---

## 🙏 致谢

- **Google SigLIP 2** —— 开源的对比式语言-图像预训练模型（通过 ONNX 使用）
- **EasyOCR** —— 轻量、多语言的 OCR 引擎
- **ONNX Runtime** —— 跨平台、硬件加速的推理框架
- **Tauri** —— 安全、轻量的桌面应用框架
- **ZeroMQ** —— 可靠、高性能的进程间通信
- **Prost** —— Rust 中符合习惯用法的 Protobuf 支持
- **Rust 社区** —— 打造了一门安全与性能并存的语言

---

## 📞 联系与支持

- **问题反馈与 Bug 报告**：[GitHub Issues](https://github.com/bobgsning/FrameScout/issues)
- **一般问题**：<bobgsning@outlook.com>
- **安全问题**：请直接邮件联系（可应要求提供 PGP 密钥）

> 🤝 **我们需要你的帮助！** FrameScout 目前是一个人的项目。如果你对隐私优先的 AI 工具充满热情，我们欢迎你的贡献——无论是代码、文档、Bug 报告，还是仅仅在你的机器上帮忙测试。
> 我们正在准备第一批 "good first issues"。在此期间，欢迎随时[发起讨论](https://github.com/bobgsning/FrameScout/discussions)或[浏览代码库](https://github.com/bobgsning/FrameScout)。

---

**FrameScout —— 你的图片。你的隐私。你的搜索。**

🔒 100% 离线 · ⚡ 毫秒级搜索 · 🖥️ 本地 AI 推理
