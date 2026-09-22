# FrameScout-UI 前端架构重构说明

这个原本 700+ 行的 `App.vue` 单文件承担了**“全局 SVG 资产、启动加载屏、授权弹窗、扫描监控总线、智能文件夹、视觉聚类、多模态搜索控制台、卡片瀑布流/折叠器、分页系统”**等多重职责。
按照 Vue 3 + TypeScript 的现代工程化标准，我们将其按**“数据类型 (Types) → 纯函数工具 (Utils) → 业务状态逻辑 (Composables) → 全局样式 (Styles) → 细粒度 UI 组件 (Components)”**进行彻底拆解。
拆分后，`App.vue` 仅作为顶级协调器（Orchestrator），代码量将缩减至 60 行以内。

```text
src/FrameScout-UI/src/
│
├── types/                        # 1. 纯 TypeScript 类型定义
│   ├── search.ts                 # SearchResult, ClusterGroup, SmartFolder 接口
│   └── license.ts                # LicenseStatus, EngineStatus 接口
│
├── utils/                        # 2. 纯计算与通用工具函数 (无状态)
│   ├── score.ts                  # mapToHumanScore, formatScore, getNumericScore
│   ├── highlight.ts              # highlight() 搜索关键词高亮逻辑
│   ├── media.ts                  # isVideo, getAssetUrl (convertFileSrc 包装)
│   ├── videoTimers.ts            # 视频加载超时注册表（模块级单例）
│   └── api.ts                    # saveNote, runOcrForItem, parseLanguages
│
├── styles/                       # 3. 样式与主题分离
│   ├── variables.css             # 7 大品牌渐变色定义、Pro/Trial 主题变量
│   ├── animations.css            # spin, pulse, slideDown, fadeIn, scaleIn 动画
│   └── common.css                # 通用按钮、输入框、标签、排版基础样式
│
├── composables/                  # 4. Vue 3 业务逻辑切片 (状态管理 & 后端 IPC)
│   ├── useEngineStatus.ts        # 监听 'engine-status'、连接重试、就绪状态
│   ├── useLicense.ts             # 授权状态检查、激活弹窗、Pro 验证
│   ├── useScanner.ts             # 目录/文件扫描、'scan-progress' 总线监听、新文件提醒
│   ├── useSearch.ts              # 多模态搜索、以图搜图、防竞态条件处理、分页计算
│   ├── useSmartFolders.ts        # 智能文件夹 CRUD 与原生后端执行
│   └── useClustering.ts          # 视觉相似度聚类阈值调节与请求
│
├── components/                   # 5. 视图与交互组件
│   ├── common/
│   │   └── SvgDefs.vue           # 全局隐藏 SVG 滤镜、渐变与 Logo Symbol 库
│   ├── splash/
│   │   └── SplashScreen.vue      # 启动时神经网络建立与连接等待屏
│   ├── header/
│   │   ├── BrandHeader.vue       # 顶部 Logo、标题与授权状态胶囊按钮
│   │   └── LicenseModal.vue      # Pro 授权激活对话框
│   ├── scan/
│   │   ├── TopActionBar.vue      # 路径输入、Browse、扫描类型、OCR 控制、幽灵清理
│   │   ├── ExtractionBus.vue     # 实时抽取总线监控面板、进度条与当前文件
│   │   └── IncomingBanner.vue    # 浮动新入库文件提示条
│   ├── smart-folders/
│   │   └── SmartFolderBar.vue    # 智能文件夹标签栏与匹配数徽章
│   ├── cluster/
│   │   └── ClusterView.vue       # 视觉相似聚类网格视图与阈值滑块
│   ├── search/
│   │   ├── SearchConsole.vue     # 搜索过滤复选框、视觉目标 Banner、主搜索输入框
│   │   └── PaginationBar.vue     # 上下页、Jump 跳转、Show All 模式切换条
│   └── cards/
│       ├── ResultCard.vue        # 结果卡片外壳（统一分发 Video/Image/Missing）
│       ├── VideoCard.vue         # 视频播放器卡片（带加载超时容错与时间戳跳转）
│       ├── ImageCard.vue         # 图片卡片（带低置信度折叠手风琴效果）
│       ├── OcrPanel.vue          # OCR 提取文本展示、折叠展开与即时识别按钮
│       └── NotePanel.vue         # 用户 Markdown 笔记展示与失焦自动保存
│
└── App.vue                       # 6. 极简主入口（仅组装各模块）
```
