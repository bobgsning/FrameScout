# 🔍 FrameScout —— 离线 AI 搜索

<p align="center">
  <img src="src/FrameScout-UI/app.svg" alt="FrameScout Logo" width="150" />
</p>

<p align="center">
  <a href="./README.md">English</a> ·
  <a href="./README.zh-CN.md"><strong>中文</strong></a>
</p>

## *100% 私有、完全离线、多模态桌面搜索引擎*

**用自然语言、OCR 文字或视觉相似度搜索你本地的图片与视频——无云端、无遥测、无账号。免费且开源（全球版）。**

[![License: Apache 2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4)](https://www.microsoft.com/windows)
![Architecture](https://img.shields.io/badge/architecture-Rust%20%2B%20Python%20%2B%20Vue-ff69b4)
[![AI-Assisted](https://img.shields.io/badge/AI--assisted-development-brightgreen)](AI_POLICY.md)

FrameScout 是一个**完全离线、由 AI 驱动的视觉搜索引擎**，面向你本地的图片与视频。
输入 *"海边和朋友一起看的日落"*、粘贴一张参考图，或是按图片中的文字来搜索——结果在毫秒之间呈现。所有计算都在你的机器上完成，没有任何数据离开你的电脑。

⬇️ [下载最新版本](https://github.com/bobgsning/FrameScout/releases) ·
📖 [快速开始](#-快速开始开发) ·
🤝 [参与贡献](CONTRIBUTING.md)

> 🆕 **v3.3.0 已发布！** 迄今最大的一次版本更新——逻辑搜索重构 + 数据安全闭环：
> - 🧮 **条件式搜索**——把查询写成看得见的条件行（关键词 / 日期 / 批次 / 文件类型 / 基准图），以「且 / 或 / 排除」组合，并支持逐条件阈值。输入内容按字面检索，无需记忆符号语法：`e-mail`、`-5°C`、`会议@2024` 均按原样匹配。智能文件夹现在保存完整条件集，与搜索栏共用同一套求值器。
> - 💾 **备份与恢复**——一键生成带时间戳的备份（自动轮转）+ 完整性校验，并支持应用内恢复：合法性预览（含帧数对比），恢复前自动备份当前库。
> - 🗂️ **分区搜索**——按来源批次、根目录前缀或入库时间范围过滤结果（以图搜图同样支持）。
> - 🧹 **自动清理策略**——事件保留与备份轮转均可设置开关，启动时给出清理报告，绝不静默删除。
> - 🌐 **真正的 i18n**——后端只回稳定错误码，文案交由前端 9 语渲染；简体中文作「文字识别」，繁体中文作「文字辨識」。
> - 🎞️ **FFmpeg 兜底 + 切片段**——OpenCV 打不开的 MKV/AVI 容器回退到随包分发、经 SHA-256 校验的 LGPL 版 FFmpeg；可从卡片或大图预览直接截取命中帧前后的一小段视频。
>
> [查看更新日志](CHANGELOG.md#330---2026-10-18) · [发布说明](https://github.com/bobgsning/FrameScout/releases)
>
> 🌐 **多语言界面**：9 种界面语言——English、简体中文、繁體中文、日本語、Русский、한국어、Deutsch、Français、Español，英文为默认语言。可随时在 **设置 → 语言** 中切换。
>
> 🔤 **多语言 OCR**：4 种内置识别（English、简体中文、繁體中文、日本語）+ 6 种按需下载识别模型（Русский、한국어、Deutsch、Français、Español，以及一个英文优化变体），更多语言即将推出。详见 [本地化与 OCR 语言](#-本地化与-ocr-语言)。

---

> **只想上手试试？** 从 [下载页](https://github.com/bobgsning/FrameScout/releases) 下载最新的 Windows 可执行文件即可，无需安装。解压后直接运行。预编译版内含 **100 帧**的免费试用额度（视频按抽帧数计入）；源码以 Apache 2.0 完全开放——试用限制同样在源码中，你可以修改或移除它，然后[构建自己的版本](#-快速开始开发)。

---

## 🧮 条件式搜索 —— 像你思考那样检索

大多数"AI 搜索"只让你打一句话然后碰运气。FrameScout 让你**把查询写成看得见的条件行**，再用**且 / 或 / 排除**组合，每行还能设**独立阈值**：

- **关键词** —— 按含义、OCR 文字、笔记或文件名匹配
- **日期** —— 例如"只看 2022 年的"
- **批次** —— 文件所属的来源导入
- **类型** —— 图片或视频
- **基准图** —— 找与参考图视觉相似的内容

想找"东京那趟、带红色招牌、但没人的"？那是三行条件，不是一句祈祷。而且**没有符号语法要记**——你输入的一切都按字面匹配：`e-mail`、`-5°C`、`会议@2024` 都按原样搜索。智能文件夹保存的正是同一套条件集、复用与搜索栏相同的求值器，所以智能文件夹说白了就是一条「存下来的查询」——点开即按当前图库重新求值，无需再手搓一遍条件。

---

## 🎯 问题所在

你的硬盘里散落着数百 GB 的截图、照片和视频片段。
现有的解决方案逼你在两个糟糕的选项之间二选一：

☁️ **把所有东西上传到云端** —— 从此永远失去隐私。
📁 **继续用本地文件管理器** —— 然后花几个小时在文件夹里翻找。

FrameScout 提供了**第三条路**：完全在本机上运行的 AI 搜索。不上传、不订阅、不妥协。

---

## 🛠️ 已知限制

我们信奉透明。以下是 FrameScout 目前**还做不到**的事，以及我们打算怎么做：

| 限制 | 应对方案 / 未来计划 |
| ---------- | ------------------------ |
| 仅支持 Windows 10/11（暂不支持 macOS / Linux） | 未来计划提供跨平台构建 |
| FlatVector 搜索为 O(N·d) 复杂度 | 当 N > 50K 时迁移到 Faiss / Annoy |
| 无文件系统实时监控 | 目前需手动重新扫描；计划引入 inotify / watchdog |
| 无 LLM 生成的图片描述 | 刻意而为——那需要联网 |
| OCR 覆盖 4 种内置 + 6 种可下载识别模型（5 种新语言 + 1 个英文优化变体），另有 6 种规划中 | 计划从官方模型库引入更多 RapidOCR rec 权重 |
| 按需 OCR 经卡片或多选候选集触发，暂无「全库 OCR」入口 | 计划提供全库 OCR 入口 |
| PDF/DOCX 暂不支持直接入库 | 请先转成 .txt/.md；计划引入文本提取 |
| 应用内数据库恢复需重启应用后生效（待恢复标记），无法即时热替换 | 计划支持会话内无缝恢复 |
| 智能文件夹已支持完整条件行（且/或/排除/阈值/日期/批次/基准图），但后端无法为条件行文件夹预计算计数（显示为「?」） | 计划提供条件行文件夹的服务端计数 |
| FTS5 全文搜索仅覆盖导入的纯文本条目（未覆盖 OCR / 文件名 / 笔记） | 计划扩展覆盖范围 |
| 视频抽帧上限（帧率 / 最大帧数 / 最大时长）为固定常量 | 计划提供可调设置项 |
| 混合融合权重为固定常量（相关度灵敏度提供宽松 / 标准 / 严格三档可调） | 计划提供逐通道权重调节 |

---

## ✨ 功能特性

| 功能 | 描述 |
| --------- | ------------- |
| 🔒 **100% 离线** | 无需联网。模型已随包内置。你的数据永不离开你的电脑。 |
| 💡 **语义搜索（ONNX + SigLIP 2）** | 输入文字，按"含义"而非仅文件名找到匹配的图片。 |
| 🖼️ **以图搜图** | 拖入一张参考图，在你的图库中找到视觉上相似的图片。 |
| 🔍 **OCR 文字搜索** | 从图片中提取并建立文字索引（英文、简体中文、繁體中文、日本語 内置；更多语言可下载）。搜索 *"三月的收据"* 就能立刻找到。支持对选中文件按需执行 OCR。 |
| 📋 **显示全部模式** | 加载所有已索引的文件。新文件按入库时间排序，始终置顶显示。 |
| 🎬 **视频帧索引** | 自动从视频中抽取关键帧（场景检测），并与静态图片一同建立索引。 |
| 🧩 **视觉聚类** | 一键发现相似图片分组（重复、近似重复、连拍）。 |
| 🧮 **条件式搜索** | 把查询写成看得见的条件行——关键词 / 日期 / 批次 / 文件类型 / 基准图——以「且 / 或 / 排除」组合，支持逐条件阈值，无需背诵符号语法。 |
| 📁 **智能文件夹（Smart Folders）** | 把一次搜索保存为具名、持久的文件夹——可以是简单查询词，也可以是多行条件（日期 / 批次 / 基准图，且/或/排除组合），每次打开时对当前图库重新求值。支持 JSON 导入 / 导出。 |
| 💾 **数据库备份与恢复** | 一键带时间戳备份（自动轮转）+ 完整性校验 + 应用内恢复（带预览，恢复前自动备份）。 |
| 🎞️ **视频切片段** | 从结果卡片或大图预览，直接截取命中帧前后的一小段视频。 |
| 📝 **个人笔记** | 给任意图片附加 Markdown 笔记，笔记可被搜索，下次更易找到。 |

### 🎬 视频帧语义搜索与即时跳转

FrameScout 不止能找到视频——它还能精确定位**具体的时间点**：

- **语义匹配**：用自然语言搜索（例如 *"栈桥下的日落"* 或 *"屏幕上的代码截图"*）。
- **一键即时跳转**：点击任意搜索结果，直接跳转到视频中的那一秒（`01:23:45`）。
- **稳健解码**：OpenCV 打不开的容器（部分 MKV/AVI 编解码器）回退到随包分发、经 SHA-256 校验的 LGPL 版 FFmpeg——不再出现静默的 0 帧索引。

---

## 🌐 本地化与 OCR 语言

FrameScout 内置 **9 种界面语言** 与 **三档 OCR 识别语言**，均在 **设置** 中配置：

**界面语言（9 种）**——在 *设置 → 语言* 中即时切换：English、简体中文、繁體中文、日本語、Русский、한국어、Deutsch、Français、Español（英文为默认）。

**OCR 识别语言（三档）**——在 *OCR 识别语言* 列表中任意组合勾选：

| 档次 | 语言 | 说明 |
| ---- | ---- | ---- |
| 🟢 内置识别 | English、简体中文、繁體中文、日本語 | 随应用附带；繁體中文与日文模型可删除 |
| 🔵 可下载识别 | Русский、한국어、Deutsch、Français、Español + English（优化版） | 模型按需下载，带进度、断点续传、SHA-256 校验与磁盘空间检查；可随时删除 |
| ⚪ 即将推出 | Português、Italiano、Polski、Türkçe、Tiếng Việt、Bahasa Indonesia | 规划中——仅列出，暂不可用 |

OCR 识别模型取自官方 [RapidAI/RapidOCR](https://github.com/RapidAI/RapidOCR) 模型库（逐项核对官方 SHA-256），存放于 `models/ocr/`。语言元数据集中在单一事实源 `src/inference-worker/ocr_manifest.json`。

## 📁 仓库结构（关键目录）

```text
FrameScout/
├── src/
│   ├── FrameScout-UI/               # Vue 3 + Tauri 桌面应用（Rust 核心）
│   │   ├── src/                     # 前端：components / composables / i18n（9 语）/ utils
│   │   └── src-tauri/               # Rust 后端：commands / storage / services / model_code
│   ├── inference-worker/            # Python AI 推理引擎（ONNX + SigLIP 2 + RapidOCR）
│   │   └── models/                  # 由 scripts/download_models.py 下载（git 忽略）
│   │       ├── siglip2-base/        # SigLIP 2 ONNX 模型（视觉 + 文本）
│   │       ├── bge-m3/              # BGE-M3 文本向量（可选）
│   │       └── ocr/                 # 各语言 OCR rec 模型（en_opt / ru / ko / de / fr / es 按需下载）
│   ├── proto/                       # ZeroMQ 通信协议（Protobuf）
├── scripts/                         # 工具脚本（模型下载、i18n 校验等）
├── README.md
├── README.zh-CN.md
├── CONTRIBUTING.md
├── CHANGELOG.md
├── THIRD_PARTY_NOTICES.md           # 第三方组件与模型权重声明（含随包 FFmpeg）
├── COPYING.LGPLv2.1                 # LGPL v2.1 完整文本（随包分发的 FFmpeg）
├── AI_POLICY.md
└── LICENSE
```

> OCR（RapidOCR / PP-OCRv4）权重随 `rapidocr_onnxruntime` wheel 内置，覆盖英文与简体中文。繁體中文与日文识别模型随安装包内置，其余语言按需下载到 `models/ocr/`。详见 [本地化与 OCR 语言](#-本地化与-ocr-语言)。

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
│  │ RapidOCR（文字提取，离线）                        │   │
│  └──────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────┐   │
│  │ OpenCV（场景检测视频抽帧）                        │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### 为何采用这样的架构？

| 决策 | 理由 |
| -------- | ------ |
| **Rust + Tauri**（而非 Electron） | 原生 WebView，二进制体积约为 Electron 的 1/10，内存占用仅为很小一部分 |
| **Python 工作进程**（而非全 Rust） | SigLIP 2 与 RapidOCR 在 Python 生态中已非常成熟，无需重复造轮子 |
| **ONNX Runtime**（而非 PyTorch） | 通用 GPU 加速（NVIDIA CUDA、AMD DirectML、Intel、CPU 回退）；体积更小 |
| **ZeroMQ + Protobuf**（而非 HTTP） | 微秒级延迟、类型安全、可完全离线工作 |
| **FlatVector 矩阵**（而非 Faiss） | 当 N < 50,000 时，O(N·d) 暴力检索已足够快（约 40ms）且零依赖 |
| **SQLite + WAL 模式** | 久经考验、嵌入式、支持写入期间并发读取 |

### 数据流

**索引一个文件夹**：Rust 遍历所选目录，收集文件路径，并通过 ZMQ 分批发送给 Python 工作进程。Python 为每张图片或视频帧生成 SigLIP 2 图像嵌入（768 维）与 OCR 文字，然后返回结果。Rust 将一切存入 SQLite 与内存中的 FlatVector 矩阵，同时前端显示实时进度。

**按文字搜索**：前端将查询发送给 Rust，Rust 转发给 Python 生成 SigLIP 2 文本嵌入。随后 Rust 用点积相似度在 FlatVector 矩阵中检索，并把原始的各通道得分（语义 / OCR / 笔记 / 文件名）返回前端，由前端归一化后融合为最终排序结果。

**按图片搜索**：流程与文字搜索一致，只是 Python 从参考图生成 SigLIP 2 图像嵌入，而非文本嵌入。

---

## 🔬 关键技术亮点

### 混合评分模型

FrameScout 将**四个信号**融合为一个统一的相关性得分。自 v3.2.0 起，后端只返回**各自独立计算的原始信号**——SigLIP 余弦相似度与各字面通道的命中标志——再由前端逐通道归一化后融合，语义匹配不再被文件名匹配淹没：

```text
fused_score = sigmoid(cosine)   × semantic_weight  （语义含义）
            + ocr_hit           × ocr_weight        （图片中的文字）
            + note_hit          × note_weight       （用户标注）
            + filename_hit      × filename_weight   （路径 / 文件名匹配）
```

归一化与融合在前端完成（`utils/score.ts`），每个通道都可在界面中独立开关。结果实时评分、稳定排序（带 tie-breaker）并分页。*注：融合权重目前为固定常量；相关度灵敏度（宽松 / 标准 / 严格）可在设置中调整——见「已知限制」。*

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

> **免责声明**：以上为开发环境的基准测试结果。实际性能取决于你的硬件与数据集特征。可用 `python src/inference-worker/scripts/benchmark.py --library <path>` 在你的机器上复现（输出 `benchmarks/YYYY-MM-DD.json`）。

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
# 3. 下载 SigLIP 2 + BGE-M3 模型（一次性，总计约 1.5GB）（在 src/inference-worker 下执行）
python ../../scripts/download_models.py

# 4. 构建并运行（全球版）
# 回到前端目录进行构建
cd ../FrameScout-UI
npm run tauri dev          # 开发模式
# npm run tauri build      # 生产构建
```

### 打包推理工作进程

Python 推理工作进程必须被编译为独立可执行文件，供 Tauri 启动。
首选 **Nuitka**（无 Temp 解压延时、体积瘦 30~50%、启动毫秒级）；PyInstaller 为 legacy 备选。

```powershell
# 在仓库根目录执行
cd src/inference-worker
# 先激活你的虚拟环境，然后：
.\build_nuitka.ps1            # 发布版（standalone + onefile）
# .\build_nuitka.ps1 -Onedir # 调试版（仅 standalone，编译更快）
```

脚本会自动处理 DirectML `capi` DLL、wheel 自带的 RapidOCR 权重、外部 `models\` 资源，
并部署到 `..\FrameScout-UI\src-tauri\bin\ai_worker\`。详见 `src/inference-worker/BUILD.md` §4。

> **注意**：打包后的工作进程会在自身所在位置相对的 `./models/` 目录中查找模型；
> Nuitka 脚本会自动把 `models\` 复制到 exe 同级。

### 首次启动

1. 出现启动画面：*"FRAME SCOUT NEURAL LINK ESTABLISHING..."*
2. SigLIP 2 + RapidOCR + BGE-M3 模型载入内存（耗时因硬件而异）

> **注意：** 预编译包**仅内置 SigLIP 2 + RapidOCR**——BGE-M3 **不在包内**。BGE-M3 文本模型只有你另行下载后才会加载；没有它，图片与视频搜索开箱即用、完全不受影响。
3. 你将看到带搜索控制台的主界面
4. 点击 **Browse（浏览）** → 选择一个包含图片的文件夹
5. 点击 **Start Indexing（开始索引）** → 观看实时抽取进度
6. 在搜索栏输入描述 → 毫秒级获得结果

---

## 🗺️ 路线图

> **关于诚实度**：FrameScout 目前由一个小团队持续开发。下方条目均已实现并可正常使用，但个别仍处于"第一版"状态——详细缺口已在上方[「已知限制」](#-已知限制)中列出。所有标注"规划中"的内容确实尚未构建。

### v3.2.0（2026-10-08）

- [x] 核心搜索（文字 + 图片 + OCR + 笔记）
- [x] SigLIP 2 迁移（768 维嵌入，ONNX Runtime）
- [x] 混合评分归一化（语义不再被文件名匹配淹没）
- [x] 视觉聚类（medoid 代表、大库采样）
- [x] 智能文件夹（持久化、可重命名、真实计数、删除二次确认）—— *当前为单查询范围*
- [x] "显示全部"模式（无分页的扁平列表）
- [x] 按索引时间排序（新文件始终置顶）
- [x] RwLock 优化（扫描期间搜索不阻塞）
- [x] 查询预处理（trim + NFKC + 切词 + 排除词）
- [x] 场景检测视频抽帧 + 相邻帧去重
- [x] FTS5 trigram 容错文本搜索 —— *仅覆盖纯文本条目*
- [x] Lightbox 大图预览（缩放 / 旋转 / 全屏 / OCR 命中红框）
- [x] 右键打开 / 在资源管理器中显示 / 复制路径
- [x] 拖拽即搜
- [x] 扫描暂停/取消 + ETA + 失败文件重试
- [x] 多选 + 候选集 + CSV/Markdown/JSON 导出
- [x] 全局 Toast 通知 + 键盘快捷键（Esc / `/` / 方向键 / Space）
- [x] 搜索历史 + 模式去黑话
- [x] 精确到帧的视频笔记 + Markdown 笔记渲染
- [x] 数据库备份 / 完整性校验 —— *单向（暂无应用内恢复）*
- [x] 数据时间线 + "去年的今天"
- [x] RRF 跨通道融合（`search_unified`）
- [x] RPC 超时收敛（不再无限卡死）
- [x] 统一搜索接入前端 + 各通道得分来源条
- [x] 设置页 + 偏好持久化（路径 / 每页条数重启后保留）
- [x] 扫描停止按钮 + 报告区分「中止」与「完成」
- [x] 离线盘幽灵保护（离线盘灰显、禁用真删除）
- [x] 批量加笔记（一次给多个选中文件写同一条笔记）

### v3.3.0（当前发布版 —— 2026-10-18）

主题：**逻辑搜索重构 + 数据安全闭环**。完整记录见 [CHANGELOG.md](./CHANGELOG.md#330---2026-10-18)。

**搜索**

- [x] 条件式搜索——看得见的条件行（关键词 / 日期 / 批次 / 文件类型 / 基准图），且 / 或 / 排除 + 逐条件阈值；输入按字面检索（无需记忆符号语法），`e-mail`、`-5°C`、`会议@2024` 均按原样搜
- [x] 智能文件夹保存**完整条件集**（`criteria_json`，迁移 M011），与搜索栏共用同一求值器；已有文件夹沿原路径兼容
- [x] 智能文件夹 JSON 导入 / 导出，按名去重，绝不静默覆盖
- [x] 分区搜索——来源批次 / 根目录前缀 / 入库时间范围过滤（以图搜图同样支持）
- [x] 排序选择器——按相关度 / 入库时间 / 文件名，跨页全局有序
- [x] 「试算」按钮——只报命中数、不动结果区
- [x] 搜索可解释性——字面命中诚实显示 ✓（不再编造百分比）、相关度灵敏度可调（宽松 / 标准 / 严格）、低分折叠可关、查询规则明示
- [x] 退役遗留的 `search_images` / `execute_smart_folder` 路径——一套引擎、一套语义

**数据安全**

- [x] 应用内数据库恢复——合法性校验、帧数对比预览、恢复前自动备份（下次启动生效）
- [x] 自动清理策略——事件保留与备份轮转提供开关，启动时给出报告；`-1` 表示「永不」
- [x] 文本入库真预演——事前说准「将替换 N 块」（`deleted_old_chunks`），不再用全库总数充数
- [x] 明确的错误语义——「发现但全部解码失败」（`ERR_SCAN_ALL_FAILED`）不再冒充「无新文件」

**体验**

- [x] 扫描暂停 / 继续，ETA 修正（暂停时长不计入已用时间）；扫描报告支持逐文件重试
- [x] 首屏自动加载浏览列表；搜索状态随会话恢复（只恢复界面，绝不自动搜索）
- [x] 来源批次人类化显示（「日期 + 文件数」）；扫描报告按批次 / 类型过滤
- [x] 以图搜图不再清空搜索框；`enableOcr` 选择持久化

**i18n**

- [x] 后端错误码契约——`error.rs` 稳定码（`CODE|参数`）交由 9 语文案渲染，中文界面不再裸露英文
- [x] 简体中文「文字识别」/ 繁体中文「文字辨識」术语统一（英文保留 OCR）
- [x] 9 语文案全部补齐（ja/ru/ko/de/fr/es 各 +92 键，本版新增 +32 键），由 `scripts/check_i18n_backend.mjs` 校验

**视频**

- [x] FFmpeg 兜底解码——随包分发、经 SHA-256 校验的 LGPL 构建，覆盖 OpenCV 打不开的 MKV/AVI 容器，带 30s 读超时看门狗
- [x] 视频切片段（`cut_video_clip` + `ClipDialog`），输出保留源容器格式
- [x] 索引 RPC 动态超时 + socket linger 卫生处理——不再残留野进程

**修复**——基准图组合条件恒 0 结果、以图搜图假 99.9 分、日期快捷区间算到未来、重命名清空条件行、暂停计时跨扫描污染、`root_path` 目录边界误配、locale 重复键等。

### v3.4+（规划中）

- [ ] 当 N > 50K 时集成 Faiss
- [ ] 端口冲突自动 fallback（worker 绑定失败时自动降级到下一个端口）
- [ ] FTS5 覆盖 OCR / 文件名 / 笔记
- [ ] 多模态融合搜索（文字 + 图片同时进行）
- [ ] 二进制 PDF/DOCX 文本提取
- [ ] 可调的视频抽帧上限
- [ ] 会话内无缝数据库恢复（无需重启）
- [ ] 条件行智能文件夹的服务端计数
- [ ] 全库 OCR 入口（不限于选中文件）
- [ ] macOS 构建（Apple Silicon 原生）
- [ ] Linux 构建（AppImage + Flatpak）
- [ ] 文件夹树侧边栏视图
- [ ] 单图删除（而非仅批量"幽灵清除"）
- [ ] 超大库虚拟滚动

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

FrameScout 核心基于 **Apache License 2.0** 授权。详见 [LICENSE](LICENSE)。第三方组件见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

© 2026 AetherFlow Labs

---

## 💎 Pro 许可证 —— 致谢密钥

为**感谢支持**，也为了方便调试，这里提供一把致谢性质的 **FrameScout Pro** 许可证密钥，可解除 100 帧试用限制，使用完整功能。

- **有效期至**：2027-01-01（锁定 v3.3.0 版本）
- **许可证邮箱/标识**：`framescout-offline_ai_search-global`
- **激活方式**：打开 **设置 → 许可证**，粘贴下方密钥并确认。

```text
ZnJhbWVzY291dC1vZmZsaW5lX2FpX3NlYXJjaC1nbG9iYWx8RlJBTUVTQ09VVF9QUk9fVjN8My4zLjB8MTc5ODc5MTYyMTo6YvXj2Vqs8SEOhkzJ5KqYRwfKkv2Gbs9uo0ZKA6MXHpDg96Uu7JNcyqjWRozYT01KmL5Vq7SkB6vDRsvdJkG5Dg==
```

> 本密钥按现状提供，仅用于支持与调试。过期后应用回退到内置试用（100 帧）；试用限制本身写在源码里，基于 Apache 2.0 从源码构建时可自行修改或移除。商业授权事宜请联系作者。

🙏 **感谢支持！** · 感谢支持！· 感謝支持！· ありがとうございます！· Спасибо за поддержку！· 지원해 주셔서 감사합니다！· Danke für deine Unterstützung! · Merci pour votre soutien ! · ¡Gracias por tu apoyo!

---

## ™️ 商标

"FrameScout"、"FrameScout —— Offline AI Search" 以及 FrameScout 徽标均为 **Bob G. S. Ning** 的商标。

其他所有商标归各自所有者所有。

---

## 🙏 致谢

- **Google SigLIP 2** —— 开源的对比式语言-图像预训练模型（通过 ONNX 使用）
- **RapidOCR** —— 轻量、多语言的 OCR 引擎
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

> 🤝 **我们需要你的帮助！** FrameScout 目前是一个小团队的项目。如果你对隐私优先的 AI 工具充满热情，我们欢迎你的贡献——无论是代码、文档、Bug 报告，还是仅仅在你的机器上帮忙测试。
> 我们正在准备第一批 "good first issues"。在此期间，欢迎随时[发起讨论](https://github.com/bobgsning/FrameScout/discussions)或[浏览代码库](https://github.com/bobgsning/FrameScout)。

---

**FrameScout —— 你的图片。你的隐私。你的搜索。**

🔒 100% 离线 · ⚡ 毫秒级搜索 · 🖥️ 本地 AI 推理
