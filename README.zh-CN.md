# 🔍 FrameScout —— 离线 AI 搜索

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

> 🆕 **v3.2.0 已发布！** 一次覆盖全产品的打磨：混合评分归一化（语义搜索不再被文件名匹配淹没）、场景检测视频抽帧、FTS5 容错文本搜索、带 OCR 命中红框的 Lightbox 大图预览、右键打开/显示/复制路径、拖拽即搜、扫描暂停/取消 + ETA、多选 + CSV/Markdown/JSON 导出、数据库备份，以及一轮数据一致性收账。 [查看更新日志](CHANGELOG.md#320---2026-10-08)。
>
> 🌐 **中英双语界面**：完整的中英文国际化，英文为默认语言。可随时在 **设置 → 语言** 中切换。

---

> **只想上手试试？** 从 [下载页](https://github.com/bobgsning/FrameScout/releases) 下载最新的 Windows 可执行文件即可，无需安装。解压后直接运行。预编译版内含 **100 帧**的免费试用额度（视频按抽帧数计入）；源码以 Apache 2.0 完全开放——试用限制同样在源码中，你可以修改或移除它，然后[构建自己的版本](#-快速开始开发)。

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
| OCR 仅支持英文 + 简体中文 | 计划支持更多语言（RapidOCR 换 rec 权重） |
| 按需 OCR 仅支持单文件 | 计划于后续版本支持多文件批量 OCR |
| PDF/DOCX 暂不支持直接入库 | 请先转成 .txt/.md；计划引入文本提取 |
| 数据库备份为单向（无应用内恢复） | 恢复需手动覆盖 .db 文件；计划支持应用内恢复 |
| 智能文件夹目前只保存单个查询词 + 通道开关（尚无布尔/时间/类型/基准图等条件） | 计划提供完整条件编辑器 |
| FTS5 全文搜索仅覆盖导入的纯文本条目（未覆盖 OCR / 文件名 / 笔记） | 计划扩展覆盖范围 |
| 无分区 / 批次 / 时间范围搜索过滤 | 计划中 |
| 相关度阈值与视频抽帧上限为固定值、不可在设置中调整 | 计划提供设置项 |
| 得分来源条中字面通道占比为近似值 | 计划回传精确的逐通道得分 |

---

## ✨ 功能特性

| 功能 | 描述 |
| --------- | ------------- |
| 🔒 **100% 离线** | 无需联网。模型已随包内置。你的数据永不离开你的电脑。 |
| 💡 **语义搜索（ONNX + SigLIP 2）** | 输入文字，按"含义"而非仅文件名找到匹配的图片。 |
| 🖼️ **以图搜图** | 拖入一张参考图，在你的图库中找到视觉上相似的图片。 |
| 🔍 **OCR 文字搜索** | 从图片中提取并建立文字索引（英文 + 简体中文）。搜索 *"三月的收据"* 就能立刻找到。支持对选中文件按需执行 OCR。 |
| 📋 **显示全部模式** | 加载所有已索引的文件。新文件按入库时间排序，始终置顶显示。 |
| 🎬 **视频帧索引** | 自动从视频中抽取关键帧（场景检测），并与静态图片一同建立索引。 |
| 🧩 **视觉聚类** | 一键发现相似图片分组（重复、近似重复、连拍）。 |
| 📁 **智能文件夹（Smart Folders）** | 把一次文字搜索保存为具名、持久的文件夹，每次打开时对当前图库重新求值。 |
| 📝 **个人笔记** | 给任意图片附加 Markdown 笔记，笔记可被搜索，下次更易找到。 |

### 🎬 视频帧语义搜索与即时跳转

FrameScout 不止能找到视频——它还能精确定位**具体的时间点**：

- **语义匹配**：用自然语言搜索（例如 *"栈桥下的日落"* 或 *"屏幕上的代码截图"*）。
- **一键即时跳转**：点击任意搜索结果，直接跳转到视频中的那一秒（`01:23:45`）。

---

## 📁 仓库结构（关键目录）

```text
FrameScout/
├── src/
│   ├── FrameScout-UI/               # Vue 3 + Tauri 桌面应用（Rust 核心）
│   ├── inference-worker/            # Python AI 推理引擎（ONNX + SigLIP 2 + RapidOCR）
│   │   └── models/                  # 由 scripts/download_models.py 下载（git 忽略）
│   │       ├── siglip2-base/        # SigLIP 2 ONNX 模型（视觉 + 文本）
│   │       └── bge-m3/              # BGE-M3 文本向量（可选）
│   ├── proto/                       # ZeroMQ 通信协议（Protobuf）
├── scripts/                         # 工具脚本（模型下载等）
├── README.md
├── CONTRIBUTING.md
└── LICENSE
```

> OCR（RapidOCR / PP-OCRv4）权重随 `rapidocr_onnxruntime` wheel 内置——英文与简体中文无需额外下载。

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

归一化与融合在前端完成（`utils/score.ts`），每个通道都可在界面中独立开关。结果实时评分、稳定排序（带 tie-breaker）并分页。*注：融合权重与相关度阈值目前为固定常量——见「已知限制」。*

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
3. 你将看到带搜索控制台的主界面
4. 点击 **Browse（浏览）** → 选择一个包含图片的文件夹
5. 点击 **Start Indexing（开始索引）** → 观看实时抽取进度
6. 在搜索栏输入描述 → 毫秒级获得结果

---

## 🗺️ 路线图

> **关于诚实度**：FrameScout 目前由一个小团队持续开发。下方条目均已实现并可正常使用，但个别仍处于"第一版"状态——详细缺口已在上方[「已知限制」](#-已知限制)中列出。所有标注"规划中"的内容确实尚未构建。

### v3.2.0（当前 —— 2026-10-08）

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

### v3.3（规划中）

- [ ] 当 N > 50K 时集成 Faiss
- [ ] 端口冲突自动 fallback（worker 绑定失败时自动降级到下一个端口）
- [ ] 智能文件夹条件编辑器（布尔 / 时间 / 文件类型 / 基准图等条件）
- [ ] FTS5 覆盖 OCR / 文件名 / 笔记
- [ ] 分区 / 批次 / 时间范围搜索
- [ ] 应用内数据库恢复
- [ ] 多模态融合搜索（文字 + 图片同时进行）
- [ ] 二进制 PDF/DOCX 文本提取
- [ ] 可调的相关度阈值与视频抽帧上限
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
