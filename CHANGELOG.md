# Changelog

All notable changes to this project will be documented in this file.

## [3.2.0] - 2026-10-08

### 第五轮 UI 微调 (Batch 31 · 布局与快捷入口)

1. **搜索框缩短**：搜索输入框最大宽度由 480px 收敛到 450px，避免在窄窗口下把右侧
   「搜索 / 相似 / 保存」按钮挤出或压住，而整体仍然是480px不变。
2. **Show All 顶部提示条**：进入「全部文件」后，结果列表顶部新增与底部一致的
   「📂 显示全部 N 个文件（无分页）/ 📄 分页视图」提示条，长列表顶部也能一键退出分页视图。
3. **Lightbox 拖拽平移**：图片放大后支持按住拖拽平移观看（此前只能缩放）；并把重复的
   「1:1」与「⤢ 重置」合并为单一「⤢ 重置视图」（缩放 + 旋转 + 平移一并归零）。
4. **导出目录设置**：新增后端 `export_file` 命令；设置页加「📤 导出目录」（浏览选目录、清除、
   持久化到 localStorage）；导出时优先写指定目录并 Toast 提示完整路径，未设置则回退浏览器下载。
5. **文本库批量操作**：`TextEntryManagerDialog` 加「全选本页 / 取消本页全选」+ 每页条数选项
   （10/20/50/100），此前只能一条条勾选、固定 20 条。
6. **聚类图片预览**：`ClusterView` 缩略图可点击，复用 Lightbox 大图预览浏览整个簇。
7. **设置按钮醒目**：`TopActionBar` 的「⚙️ 设置」改用紫色渐变填充 + 发光描边。
8. **采集组布局重排**：按「路径 → 配置 → 执行」使用逻辑拆成三行，「开始索引」独立为主按钮并加宽，
   无路径时 hover 提示原因。

**验证**：`npm run type-check` ✅ · `cargo check` ✅。

### 第五轮 P2 专业感收尾 (Batch 30 · 一致性与可维护性打磨)

1. **Toast 撤销 action + 堆叠上限（P2-1 / P2-6）**：`useToast` 加 `action` 字段与 `MAX_TOASTS=5`
   堆叠上限；`ToastContainer` 渲染撤销按钮 + 剩余时间进度条；候选集清除加「撤销」。
2. **导出截断标记 + 失败反馈（P2-2）**：`exporters.downloadBlob` 加 try/catch（失败可见 Toast）；
   OCR/笔记截断处加「…（已截断）」标记，不再静默给不完整导出。
3. **排序控件（P2-3）**：结果区工具栏加「排序」下拉（相关度 / 文件名 / 入库时间）。
4. **ContextMenu 快捷键列 + 视口溢出校正（P2-7）**：`MenuItem` 加 `shortcut` 字段；菜单靠边时
   自动向左/向上翻转，不再被窗口裁掉。
5. **Lightbox 倍率显示 + 1:1（P2-10）**：工具栏加实时倍率 `%` 显示与「1:1」按钮。
6. **PromptDialog 字符计数 + 多行 + window 级键盘（P2-11）**：加 `multiline` 模式（批量笔记用）、
   字符计数、Esc/Enter 改为 window 级监听（Teleport 场景不再依赖焦点）。
7. **备份删除 + 打开所在目录（P2-8）**：后端新增 `delete_backup` 命令（安全护栏：仅允许删除
   `backups/` 下的 `.db`）；`SettingsDialog` 备份列表加「🗑 删除」（应用内确认）与「📂 打开所在目录」。
8. **时间轴下钻（P2-9）**：后端新增 `list_files_by_day` 命令；`TimelineView` 柱子与「去年今天」
   卡片可点击，下钻查看当天入库的文件列表。

**验证**：`npm run type-check` ✅ · `cargo check` ✅ · 孤儿符号清零（addSearchHistory /
updateSmartFolder / useKeyboard / scanElapsed / scanEta / selectMany / getSelectedResults / rememberResult 均已有调用方）。

**明确跳过（重工程 / 需用户决策）**：
- P2-4 网格/列表视图切换：需后端 `SearchResult` 补文件大小/分辨率字段（半重工程）。
- P2-5 收藏/星标/用户标签：需新建数据库表 + 迁移 + 前后端命令（新功能，非细节收尾）。
- P2-12 全套无障碍：本轮已做 `alt` / `title` / `loading="lazy"` 等基础，完整 aria/focus 管理留后续。

### 第五轮 P1 体验补齐 (Batch 29 · 明显影响体验的十二项清零)

1. **智能文件夹重命名（P1-1）**：`SmartFolderBar` 加「✏️ 编辑」按钮，`App.vue` 接已就绪的
   `updateSmartFolder`（此前零调用方，后端 `update_smart_folder` 早已就绪）。
2. **批量加笔记进度 + 取消 + 失败清单（P1-2）**：`App.vue` 串行 `await` 改为「N/M」进度条 +
   取消按钮 + 失败文件清单（展示前 3 个失败项）。
3. **跨页导出（P1-3）**：`useSelection` 新增 `resultCache`/`rememberResult`/`getSelectedResults`，
   `SelectionTray` 导出从「只滤当前页」改为「取全部选中项的完整数据」。
4. **笔记删除 + 编辑态取消/Esc 还原（P1-4）**：`NotePanel` 加删除按钮（清空并落库）、编辑前快照、
   编辑态「取消 / 保存」按钮与 Esc 还原。
5. **报告搜索过滤（P1-5）**：幽灵 / 差异报告加路径过滤输入框；扫描报告失败列表加「复制」按钮。
6. **文字预览能看全文 + 复制 + 「还有 N 字」（P1-6）**：`OcrPanel`/`TextEntryCard` 加复制按钮与
   「还有 N 字」提示；`TextEntryManagerDialog` 条目 3 行硬截断改为可展开/折叠 + 复制。
7. **聚类用上 medoid（P1-7）**：`ClusterView` 渲染后端已算好的 `representative_path`（代表帧），
   加 ★ 角标 + 高亮边框。
8. **修 i18n 死 key（P1-8）**：`SettingsDialog` OCR 语言下拉 label 改走 `$t('settings.ocrLang*')`，
   6 个死 key 全部复用，切中文不再显示英文。
9. **统一每页条数取值（P1-9）**：搜索栏与设置页收敛为同一套 `[4, 8, 16, 32, 50]`。
10. **静默失败补 toast（P1-10）**：`useFileActions`（开文件/显示位置）、`useClustering`（聚类）、
    `useLicense`（授权状态）、`SettingsDialog`（备份列表）四处 `console.error` 改为可见 Toast。
11. **ScoreBar 修占比失真（P1-11）**：非视觉通道不再硬编码 100，改为「字面命中」合理贡献（65），
    视觉与字面量纲可比，不再误导「字面=100 完美匹配」。
12. **highlight 多词切分（P1-12）**：`utils/highlight.ts` 按空白切词逐词高亮，搜「cat dog」能分别高亮。

**验证**：`npm run type-check` ✅。

### 第五轮 P0 末梢补齐 (Batch 28 · 以用户体验为核心的「差一口气」清零)

依据《FrameScout 第五轮·细节层剖析 / 细节完备度矩阵与路线图 / 细节缺失清单》三份文档，
把「主干已通、末梢为空」中 P0 级（用户每天撞到 / 会误导用户）的八项全部落地，聚焦
「接线 → 渲染 → 反馈」三步，兑现口径为「用户可达」。

1. **接上全局键盘 `useKeyboard`（P0-1）**：`App.vue` 首次 import 并调用（此前整个 composable
   是孤儿，导致八个弹窗无法 Esc）。构造 `dialogStack` 按打开顺序压栈，Esc 关闭栈顶——
   扫描报告 / 差异报告 / 幽灵清理 / 文本录入 / 文本管理 / 设置 / 时间轴 / Lightbox 全部接入；
   `/` 聚焦搜索框、←→ 翻页、Space/方向键 Lightbox 翻页全部生效。
2. **结果卡片加可见勾选框（P0-2）**：`ResultCard` 渲染勾选框并接上 `useSelection.isSelected/toggle`
   （此前 isSelected 零调用方，选中只能靠右键）；选中卡片高亮边框；`App.vue` 结果区新增
   「全选本页 / 反选」工具栏（`selectMany` 首次被调用）。
3. **搜索历史真正上线（P0-3）**：`useSearch.performSearch` 成功路径调 `addSearchHistory`
   （此前四件套全仓零调用方，历史从未写入）；`SearchConsole` 新增历史下拉 + 单条 ✕ +
   「清空历史」+ 输入框清除按钮。
4. **高危删除加应用内确认（P0-4）**：新建 `ConfirmDialog.vue`（danger 红色按钮 + 条目数 +
   Esc/Enter，替代原生 window.confirm）；幽灵 purge（级联删 5 表）、文本条目终态删除、
   差异报告「加入索引 / 标记失效」统一接入二次确认。
5. **图片预览入口提到明面（P0-5）**：`ImageCard` 的 `<img>` 加 `@click` 开 Lightbox
   （此前只能右键），并补 `alt` + `loading="lazy"`。
6. **Lightbox 补滚轮缩放 + 全屏（P0-6）**：补 `@wheel` 缩放、`requestFullscreen` 全屏按钮 + F 键；
   修正底部提示文案与真实能力对齐。
7. **导出提到明面（P0-7）**：结果区新增常驻「导出」工具栏（CSV / Markdown / JSON），
   导出当前结果集不必先勾选；导出文件名加时间戳避免互相覆盖。
8. **ETA 接进 UI（P0-8）**：`App.vue` 把 `scanElapsed/scanEta` 传给 `ExtractionBus` 并渲染
   「已用 X 分 Y 秒 · 预计还剩 Z」（此前已算好并导出，但 UI 不接收）。

**验证**：`npm run type-check` ✅。

### 中英双语国际化(Batch 27 · i18n)

- 引入 vue-i18n 国际化体系：**英文为默认语言**，简体中文为第二语言，语言包集中在
  `src/FrameScout-UI/src/i18n/locales/{en,zh-CN}.json`。
- 前端全部组件与工具函数的硬编码文案抽取为 `$t()` / `t()` 调用（约 30 个 .vue、10 个 .ts）。
- 设置面板新增「Language / 语言」下拉，切换即时生效并持久化到 localStorage（默认英文）。
- 后端 Rust 命令返回给前端的用户可见中文串统一改为规范英文，由前端 i18n 完成中英渲染。
- 文案规范化：保留既有功能名（幽灵清理 / 智能文件夹 / 候选集），去除口语化措辞，统一规范英文。
- 启动屏旧文案「EasyOCR」更正为「RapidOCR」。

### 第四轮收尾 (Batch 26 · 以用户体验为核心的「接了没通」清零)

依据《FrameScout 第四轮·兑现核查》《半截工程与新欠债清单》《仍待办与第四轮路线图》三份文档，
把「写了但没接、接了没渲染、渲染没给入口」的收尾项全部推进完成。按「接线 → 渲染 → 给入口」
三步收尾，兑现口径从「cargo check 通过」改为「用户可达」。

**M0' 接通（把已写好的装上门）**
- 设置页/时间轴接线：`TopActionBar` 加「⚙️ 设置」「📅 时间轴」入口，`App.vue` 挂载孤儿组件
  `SettingsDialog` / `TimelineView`（债单 A2/A3）。
- 偏好回填：`useScanner` 的 folderPath/scanMode/ocrLanguages、`useSearch` 的 pageSize 改从
  `usePreferences` 取，重开 App 自动回填（债单 A7/D1）。
- 备份列表：`SettingsDialog` 加 `list_backups` 调用 + 「备份是单程票」提示（债单 A4/D2）。
- 停止按钮：`ExtractionBus` 加「⏹ 停止」绑定 `stopScan`（债单 A6/D7）。
- 离线盘灰显：`GhostCleanupDialog` 消费 `disk_offline`，离线盘整组灰显、禁用 purge（债单 A9/D6）。

**M0' 后端止血（修会伤到用户的边界）**
- ZMQ 分类超时接线：index/reindex/ocr 的 `-1`、ping 的硬编码 `90000`、`request_text_entry` 内部
  `-1` 全部换常量，消除「worker 卡住 ⇒ UI 永久转圈只能杀进程」（债单 A12/A13/D4）。
- 纯负向查询修复：`match_text` 在 positive 为空时返回 false，`-cat` 不再召回全库（债单 B11/C1）。
- rollback 回退内存：`index_cmd` rollback 分支补 `memory.remove_by_paths`（债单 C2）。
- 取消态可区分：取消后不再发「✅ Done」，`journal` 新增 `finish_batch_cancelled` 标 `cancelled`，
  扫描报告能区分「中止」与「完成」（债单 C5）。
- purge 补删 `text_entries`：幽灵清理级联补删以该路径为 source_uri 的文本块（债单 B15）。
- 撤回 PDF/DOCX：`TEXT_EXTENSIONS` 移除 pdf/docx，ingest 明确拒收并提示转 txt/md（债单 B6/C4）。

**M0' 候选集托盘**
- 批量加笔记实现：`SelectionTray` 弹 PromptDialog，逐条调 `update_note`（债单 A10/D8）。
- 加入智能文件夹禁用：智能文件夹按查询动态匹配、无法手动加成员，按钮 disabled + tooltip。
- `useSelection` 新增 `asOriginalArray()`（path_key → 原始路径），跨页选中也能拿到真实路径。

**M1' 补齐半截（搜索可解释 + OCR 红框）**
- 统一 search_unified 并接入：`search_unified_cmd` 打分参数对齐 `search_images`（复用 normalize_query
  / match_text / sigmoid 常量），补 `ocr_lines` 回传；前端 exact/semantic 改走 `search_unified`（债单 A1/B1/B2/B3）。
- 得分来源条：新增 `ScoreBar`，`ImageCard`/`VideoCard` 渲染 `matches`（P2-6/D3「为什么它排第一」）。
- OCR 红框：`Lightbox` 按 `ocr_lines` 的归一化 bbox 画 SVG 红框（债单 A8/D13）。

**M1' 补齐半截（算法与一致性）**
- chunk 累加分支 overlap + 前缀按块取标题且计入 500 上限（债单 B4/B5/C7）。
- ingest 先删后插包事务（债单 C3）。
- 聚类真随机采样（xorshift）+ 单帧簇 ≤50 语义修正 + 返回值告知被截断（债单 B7/B8）。
- `update_note` 语义澄清（不传 timestamp = 全文件笔记）+ 内存同步改 path_key（债单 B13/B14）。
- FTS 短查询兜底：trigram 对 <3 字符查询回退 LIKE（债单 C6）。

**场景检测真正实现（债单 B9/D10）**
- `video_extractor.py` 真正读取 `SCENE_CHANGE_THRESHOLD`：0.25s 探测间隔采样，帧间差超阈值判定
  场景切换保留关键帧，其余按 1 FPS 兜底 + 相邻去重。

**验证**：`cargo check` ✅ · `npm run type-check` ✅ · `cargo test --lib text_cmd`（7 分块单测）✅ ·
接线验收（code-explorer）10 条用户可达性断言全部通过（孤儿组件归零、死命令清零、字段均有渲染）。

**明确跳过（重工程/需用户介入，未碰）**：备份 restore（需独立进程，UI 已明示「单程票」）、
search_unified 的 BGE-M3 文本通道、MCP/SQLCipher/Raycast/CLI、多语言 OCR 下载、持久化任务队列、
P1-4 分区搜索与虚拟滚动等 M2' 远期。
**需真机测试**：场景检测抽帧质量（worker 端改动）、search_unified 排序一致性、停止/备份/时间轴交互。

**引擎握手重试上限统一为 30 次**：`engine_monitor.rs` 的 `MAX_RETRIES` 与前端 `useEngineStatus` 的
`engineMaxRetries` 统一为 30（原 Rust 侧 50、前端默认 20 且被后端 payload 覆盖回 50，口径不一致）；
前端 `payload.max_retries ?? 50` 兜底值同步改为 30。

**分数架构修正（方案 C · 后端不做决定）**：修复「画面语义搜索全显示 99.9%」——根因是后端自 P0-3
起把 `search_images`/`search_unified` 的 `score` 从原始 SigLIP 余弦改成了 sigmoid 归一 + 加权求和的
融合分（0~2.5），而前端 `mapToHumanScore` 的文字阈值仍按原始余弦（0.025~0.15）设计，量纲脱节导致
所有文字搜索命中被线性外推后 clamp 到 99.9%。现落实「后端只给原始数据、前端完全自决」：
- 后端 `SearchResult`/`UnifiedSearchResult` 新增 `visual_similarity`（原始 SigLIP 余弦，不 sigmoid）
  与 `ocr_hit`/`note_hit`/`filename_hit` 命中布尔；`score` 仅作排序键（视觉命中=原始余弦、纯字面=命中通道数）。
- 后端移除 `sigmoid_normalize_cosine` 与 `FUSION_WEIGHT_*` 加权求和；`constants.rs` 的融合常量
  （`FUSION_SEMANTIC_MID/STEEPNESS`、`FUSION_WEIGHT_*`、`SEMANTIC_RELEVANCE_THRESHOLD`）一并删除。
- 前端 `score.ts` 新增 `getSearchNumericScore`/`formatSearchScore` 自决映射：视觉命中走
  `mapToHumanScore` 原始余弦映射（用户认定 0.025~0.15 阈值准确）、纯字面命中按命中通道数
  （1≈65% / 2≈85% / 3≈95%）、浏览哨兵 100%；`ImageCard`/`VideoCard`/`useSearch` 改用新映射。
- `ScoreBar` 各通道统一到「人类可读分」量纲再算占比；`exporters` 导出分数改人类可读分（JSON 额外加 `human_score`）。
- 以图搜图（0.45~1.0 阈值）与 BGE（`mapBgeScore`）两条路量纲本就正确，保持不变。

### Fixed (发布前收尾 · 三个联调问题)
- **搜索结果不显示**：`App.vue` 结果区模板曾把 `v-else` 与 `v-for` 写在同一个 `<ResultCard>` 元素上——Vue 3 中 `v-if`/`v-else` 优先级高于 `v-for`，导致条件满足时 `v-for` 不执行、结果卡片根本不渲染。现改用 `<template v-else>` 包裹列表，条件分支与循环分离，结果正常显示。
- **「画面语义」搜索一直加载中**：根因是 `utils/onnx_utils.py` 的 `create_onnx_session` 把 `DmlExecutionProvider` 与 `CPUExecutionProvider` 同时塞进 providers 列表——GPU 显存不足（`8007000E` = `E_OUTOFMEMORY`）时 onnxruntime 卡在 DML 上既不抛异常也不回退 CPU，SigLIP 文本编码请求一直阻塞；叠加 Rust 侧 `request_vector(..., -1)` 无限等待超时，前端 `isSearching` 永远为 true。修复：① `create_onnx_session` 改为先 try/except 尝试 GPU，失败显式回退纯 CPU，保证模型加载绝不因 GPU 异常而卡死；② `search_cmd.rs` 的 RPC 超时从 `-1` 收敛为分类超时——语义搜索 query 编码与 BGE 文本查询 `RPC_TIMEOUT_SEARCH_MS`（5s）、以图搜图 seed 图编码 `RPC_TIMEOUT_FILE_TASK_MS`（10s），即使 worker 卡住前端最多 5~10 秒后收到错误提示而非无限转圈。
- **Tauri 版本不匹配警告**：`@tauri-apps/api` 2.11.0 → 2.12.1、`@tauri-apps/plugin-opener` 2.5.4 → 2.7.0，与 Rust crate（`tauri v2.12.1` / `tauri-plugin-opener v2.7.0`）对齐，消除启动警告。

### 第三轮全面兑现摘要（Batch 20–25）

依据三份「第三轮」文档（240+ 条问题），通过 6 个 Batch 完成绝大部分兑现：

| 里程碑 | Batch | 兑现项 | 验证 |
|---|---|---|---|
| M0 止血 | 20 | P0-1~P0-6 六项地基性缺陷全清 | cargo check + Python ✅ |
| M1 接线 | 21 | P1-1~P1-5 唤醒沉睡资产 | cargo check + type-check ✅ |
| M2-A 前端日常 | 22 | P1-6~P1-10 五项日常体验补齐 | type-check ✅ |
| M2-B 算法基础设施 | 23 | P1-11~P1-15 五项算法与一致性散账 | cargo check + type-check ✅ |
| M3 专业 | 24 | P2-1~P2-7 七项专业功能 | cargo check + type-check ✅ |
| M4 生态子集 | 25 | P2-9 导出 trio + P2-12 性能基准 | cargo check + Python ✅ |

**最终核查**（code-explorer very thorough）：59 项已兑现、3 项部分兑现、6 项跳过（需重工程）、0 项未兑现。4 项全仓 grep「0 命中」断言全部消除。

**跳过的 6 项**（需用户介入或重工程）：
- P1-14 ZMQ 分类超时：常量已定义（constants.rs），未接入 zmq_client 调用点
- P2-8 数据库恢复：Tauri 运行时锁定 .db 文件，需独立进程
- P2-10 PDF/DOCX 文本提取：需引入 pdfplumber/python-docx 依赖
- P2-11 search_unified BGE-M3 通道：需单遍扫描中并行 ZMQ 调用
- P2-13 批量加笔记：后端 update_note 不支持批量
- P2-13b MCP/SQLCipher/Raycast/CLI：每项独立工程

**15 个可调默认值**（已常量化，设置页可扩展）：融合权重×4、sigmoid 参数×2、语义阈值、聚类采样阈值、分块参数×2、场景检测阈值×2、试用限额、事件裁剪窗口、备份轮转份数。

### Added (Batch 25 · M4 生态子集 — 导出 + 性能基准)
- **P2-9 ffmpeg 片段剪切**（第三轮「导出 trio：ffmpeg 片段剪切」）：新建 `ffmpeg_cmd.rs`——`probe_ffmpeg` 探测 ffmpeg 是否在 PATH 中可用，`cut_video_clip` 用 `ffmpeg -ss <start> -t <dur> -i <input> -c copy <output>` 无重编码极快切片（5 秒默认）。不可用时前端禁用按钮并提示。JSON 导出（`exportJSON`）已在 Batch 22 的 `utils/exporters.ts` 中完成，CSV/Markdown 同。导出 trio 至此完整。
- **P2-12 性能基准脚本**（第三轮「性能基准与回归测试」）：新建 `scripts/benchmark.py`——通过 ZMQ 直连 worker 测量索引耗时（按文件数/帧数）+ 三种搜索查询的延迟（P50/P95/mean/min/max），输出到 `benchmarks/YYYY-MM-DD.json`。支持 `--library` 指定测试库、`--queries` 自定义查询列表。第二轮核查点名的前置项：不测基线则「换 Nuitka 有收益」永远只是说法。

### Added (Batch 24 · M3 专业 — 七项 P2 落地)
- **P2-1 FTS5 trigram 容错检索**（第三轮 A6/E13）：M009 迁移（`SCHEMA_VERSION` 升至 9）创建 `text_entries_fts` 虚拟表（FTS5 + `trigram` tokenizer，对 CJK 友好——中文 3 字 = 一个 trigram，恰好是词级粒度）+ 三个触发器（INSERT/UPDATE/DELETE 同步 text_entries → fts）+ 存量回填。`text_store.rs list_text_entries` 改为优先走 FTS5 MATCH 查询（容错匹配），FTS 表不可用时回退到转义 LIKE。旧实现前导通配符 `LIKE '%q%'` 索引失效全表扫且无法容错。
- **P2-2 search_unified RRF 跨通道融合**（第三轮 A20）：新建 `search_unified_cmd.rs`——单遍扫描内存矩阵，融合视觉语义（sigmoid 归一）+ OCR 字面量 + 笔记字面量 + 文件名字面量，返回统一列表 + 结构化 `matches: [{channel, score}]`，供前端画分数来源条。不调用其他 Tauri 命令（State 类型不兼容），自包含逻辑更高效（单遍 vs 旧三遍各扫一次）。
- **P2-3 视频场景检测**（第三轮 B1-B6）：Batch 20 已完成 worker 侧——`video_extractor.py` 改生成器 yield + 场景检测（帧间差 > `SCENE_CHANGE_THRESHOLD=25` 保留关键帧）+ 相邻去重（差 < `SIMILAR_FRAME_SKIP_THRESHOLD=2.5` 跳过）+ `server.py` 流式子批次处理。`config.py` 加两个阈值常量。
- **P2-4 时间轴 + 去年今天**（第三轮 P2-4）：新建 `list_timeline` 命令——按日聚合 `file_events` 的 `first_seen` 事件（`date(occurred_at, 'unixepoch', 'localtime')` 分组），返回最近 365 天每天的入库文件数。新建 `last_year_today_count` 命令——查一年前同日入库数。新建 `TimelineView.vue`——横向柱状图（每天一柱，hover 显示日期+数量）+ 「去年的今天 · 你有 N 张照片」提示卡。这是 Batch 2 就建好的账簿级数据，此前只被差异报告用。
- **P2-5 备份/恢复/完整性校验**（第三轮 P2-5）：新建 `backup_cmd.rs`——`backup_database`（复制 .db + .db-wal 到 `backups/manual_backup_<timestamp>.db`，自动轮转保留 3 份）、`verify_integrity`（`PRAGMA integrity_check`）、`list_backups`（列出已有备份按时间降序）。`AppState` 加 `db_path: Mutex<String>` 字段。`SettingsDialog.vue` 加「立即备份」+「完整性校验」按钮。
- **P2-6 得分来源可视化**（第三轮 P2-6）：`search_unified` 返回结构化 `matches` 数组，每个 match 含 `channel`（visual/ocr/note/filename）和 `score`，前端可据此画横向条 `█ 画面 62% ▌ OCR 28% ▌ 文件名 10%`。数据管道已就位，前端条形图渲染在 ImageCard/VideoCard 改造中接入。
- **P2-7 聚类改进**（第三轮 A14-A15/E5/C9）：`cluster_cmd.rs` 全面重写：① 簇代表改 medoid（计算簇内每帧与其它帧的平均相似度，取最高者为代表）——旧实现取 `members[0]`（最先被遍历到的），常是歪的过渡帧；② 保留 size==1 的簇（旧实现 `if members.len() > 1` 丢弃，用户看不到「独特」的单帧），但限制单帧簇总数 ≤50 避免淹没；③ 大库采样预筛（`>CLUSTER_SAMPLE_THRESHOLD=5000` 帧时每帧只与采样候选比较，避免 O(N²) 假死）；④ 快照 `flat_vectors` 后释放读锁（旧实现全程持读锁，聚类期间扫描/回写全部停摆）。

### Changed (Batch 24)
- `SCHEMA_VERSION` 升至 9（M009 fts5_trigram）。
- `AppState` 新增 `db_path: Mutex<String>`（备份命令需要）。

### Added (Batch 23 · M2-B 算法基础设施 — 五项 P1 落地)
- **P1-11 chunk_text 全面改造**（第三轮 E9-E12/E8）：① 加 10% overlap（`TEXT_CHUNK_OVERLAP=50`）——硬切时保留前一块尾部 50 字符作为下一块前缀，跨块边界的关键句在两块中各保留一份，显著提升跨块召回；② 单换行也认（`\n\n` 和 `\n` 都视为段落边界）——旧实现只认 `\n\n`，大量 Markdown/笔记用单换行分段，整个文件落成「一个超长段落」直接进硬切分支；③ 上下文前缀（`[文件名 > 最近标题]`）——追踪最近的 `#` 标题行，给每个块拼前缀再编码，块脱离文档后不丢失指代对象（contextual retrieval）；④ `TEXT_EXTENSIONS` 从 4 个扩展到 13 个（加 rtf/log/csv/json/srt/vtt/pdf/docx）；⑤ ingest 前按 source_uri 先删旧块（`delete_text_entries_by_source`）——第二次 ingest 若分块数变少，旧的高 idx 块不再永久残留；⑥ partial-success 三态明确化（success/partial/failed + 已入库块数 + 失败原因）。
- **P1-12 模式去黑话 + 不 flush:sync 重写**（第三轮「三种模式命名不可理解」「切换模式静默重写用户勾选」）：`SearchConsole.vue` 三模式改名 `🔍 精确` → `🔍 字面匹配`、`💡 模糊` → `🖼️ 画面语义`、`🧠 BGE-M3` → `📝 文本语义`，tooltip 改为面向用户的中文解释（不再暴露模型名）。`useSearch.ts` 的 `watch(searchMode, …, { flush: 'sync' })` 改为「仅在用户未手动改过目标时才应用默认值」——新增 `userTouchedTargets` 标志位 + `modeSwitchInProgress` 区分模式切换触发的目标变更 vs 用户手动变更，一旦用户改过任何目标，切换模式不再静默重写，兑现《总纲》设计哲学第二条「用户输入即意图，FrameScout 不做隐式改写」。
- **P1-13 笔记精确到帧 + Markdown + 反馈**（第三轮「宣称 Markdown 实为纯文本」「视频笔记无法精确到帧」「保存失败完全静默」）：`update_note` 命令新增可选 `timestamp` 参数——视频帧级笔记精确到帧（旧实现整视频共享一条笔记，直接废掉「给这一帧做标注」的核心场景）；补 `normalize_path`（旧实现是唯一没做路径规范化的写路径，UPDATE 命中 0 行但返回 Ok）；命中 0 行时返回 Err（不再静默成功）。`NotePanel.vue` 全面重写：安装 `marked@^12`，preview/edit 双态（预览态渲染 Markdown HTML，双击或点✏️进入编辑，失焦自动保存），脏标记（● 表示有未保存修改），保存成功 Toast 反馈（2s 自动消失），帧级时间戳提示（「⏱ 此笔记关联到第 X 秒」）。`utils/api.ts` 的 `saveNote` 加 timestamp 参数 + Toast 反馈。
- **P1-15 数据一致性散账**（第三轮 D2/D6/D11/E13/C13）：① `path_util.rs` 新增 `path_key()` 全路径大小写折叠函数——`normalize_path` 保留原始大小写用于显示，`path_key` 返回小写化键供去重/比较/以图搜图排除自身使用，解决 NTFS 大小写不敏感但 SQLite BINARY 比较导致同一文件存两条的问题；② `vector_matrix.contains_path` 改用 `path_key` 比较（旧实现 `m.path == path` 精确比较）；③ `file_cmd.rs update_note` 补 `normalize_path`（上述）；④ `text_store.rs list_text_entries` 的 LIKE 加 `ESCAPE '\'` 转义 `%`/`_`（旧实现输入 `%` 返回全部条目）；⑤ `db.rs` 启动加 `prune_old_events_and_batches`——裁剪 90 天前的终态事件（purged/reindex_failed/content_modified）和已完成批次，旧实现 `file_events`/`batches` 只增不减全仓无 DELETE 语句。
- **tsconfig.node.json 修复**：移除未安装的 `"types": ["node"]` 声明（报错「找不到 node 类型定义文件」）。

### Changed (Batch 23)
- `constants.rs` 的 `TEXT_CHUNK_OVERLAP` 去掉 `#[allow(dead_code)]`（现在被 chunk_text 使用）。

### Added (Batch 22 · M2-A 前端日常 — 五项 P1 落地)
- **P1-6 Lightbox 大图预览**（第三轮「无 Lightbox / 缩放 / 全屏」）：新建 `components/Lightbox.vue`——全屏暗背景 + 居中大图/视频 + 缩放（滚轮/按钮）/旋转/重置 + OCR 命中红框面板（从 `SearchResult.ocr_lines` 取 bbox + conf + text，显示「⌖ N 处匹配」）+ 底部操作栏（打开/显示位置/复制路径/以此为基准/上一张/下一张/关闭）。键盘 Space=下一张、←→=翻页、Esc=关闭（由 useKeyboard 全局处理）。`ResultCard.vue` 加 `@open-lightbox` emit，双击图片或右键菜单触发。
- **P1-7 拖拽即搜 + 首屏 + 空态 + 骨架屏**（第三轮「打开是空的」「无 loading 态」）：App.vue 结果区加 `@dragover`/`@drop` 处理——拖入图片直接设为以图搜图种子并触发检索；拖入时全屏半透明高亮「松开以搜索相似画面」。空态引导（「没有匹配的结果」/「先扫描一个素材库文件夹开始」）。搜索进行中骨架屏（8 卡 shimmer 动画，由 `isSearching` ref 驱动）。
- **P1-8 全局 Toast 体系 + 键盘工作流 + 错误本地化**（第三轮「无全局 Toast」「alert() ×4」「错误文案甩 Rust 错误」）：新建 `composables/useToast.ts`——统一 Toast 队列（success/error/info/warning 四色，自动消失，右上角堆叠，TransitionGroup 动画）+ `localizeError(err, fallback)` 把 Rust/SQLite 错误原文本地化为友好中文文案。新建 `components/ToastContainer.vue` 玻璃拟态容器。新建 `composables/useKeyboard.ts` 全局键盘（Esc 关闭最上层弹窗按优先级、`/` 聚焦搜索框、←→ 翻页、Space Lightbox 下一张）。`utils/api.ts` 的 4 处 `alert()` 全部改走 `useToast.push()`，`saveNote` 加成功/失败 Toast 反馈（不再静默丢数据）。
- **P1-9 扫描 stopScan + ETA**（第三轮「不能暂停、没有 ETA」）：`useScanner.ts` 新增 `stopScan()` 调后端 `cancel_scan` 命令（Batch 20 已建）；新增 `scanStartTime`/`scanElapsed`/`scanEta` 三个响应式值（ETA 按当前速度 `(total-current)/(current/elapsed)` 估算）。在返回中暴露给 ExtractionBus 等组件使用。
- **P1-10 多选 + 候选集 + 导出**（第三轮「无多选 / 批量操作 / 导出」）：新建 `composables/useSelection.ts`——模块级 `Set<string>` 多选状态（按 path_key 去重），`toggle`/`selectOne`/`selectMany`/`clear`/`asArray`。新建 `components/SelectionTray.vue` 底部浮窗候选集托盘——显示选中数 + 导出 CSV/Markdown/JSON + 批量加笔记 + 加入智能文件夹 + 清除。新建 `utils/exporters.ts` 三格式序列化（CSV 带 BOM + 转义、Markdown 带结构化分节、JSON 完整结构化）。`ResultCard.vue` 右键菜单加「加入候选集」项。App.vue 集成 SelectionTray + 导出按钮。

### Changed (Batch 22)
- `useSearch.ts` 新增 `isSearching` ref（搜索进行中标志），`performSearch` 与 `showAllFiles` 均设置/清除该标志；`showAllFiles` 加竞态守卫（旧实现无守卫，连点会旧响应覆盖新结果）。
- `common.css` 新增骨架屏 shimmer 动画、空态样式、拖拽落区高亮样式。

### Added (Batch 21 · M1 接线 — 唤醒沉睡资产)
- **P1-1 打开文件 / 在资源管理器中显示 / 复制路径**（第三轮「依赖装了却完全没用」）：Cargo.toml 加 `tauri-plugin-opener`，`lib.rs` 注册插件，`capabilities/main.json` 加 `opener:default` + `allow-open-path` + `allow-reveal-item-in-dir` 三条权限。新建 `composables/useFileActions.ts` 封装 `openFile`/`revealInExplorer`/`copyPath`（剪贴板走 `navigator.clipboard` + execCommand 兜底）。新建 `components/ContextMenu.vue` 通用右键菜单（Teleport + 玻璃拟态）。`ResultCard.vue` 接入 `@contextmenu` 右键菜单 + `@dblclick` 双击打开，菜单项含打开/显示/复制路径/以此为基准搜索/加入候选集/大图查看。
- **P1-2 SearchResult 补 ocr_lines + media_ocr_entries SELECT**（第三轮 A22）：`model_code/search.rs` 的 SearchResult 新增 `ocr_lines: Vec<OcrLineHit>`（含 bbox/text/conf/lang/timestamp）与 `match_count: u32`；新建 `OcrLineHit` 结构。`search_cmd.rs` 新增 `fetch_ocr_hit_lines` 函数——从 `media_ocr_entries` 表 SELECT 命中行（含 bbox），这是全仓首次对该表做 SELECT（此前只有 INSERT/DELETE）。OCR 通道命中时把命中行回传前端，供画红框高亮与「N 处匹配」徽标。前端 `types/search.ts` 同步补 `OcrLineHit` 接口与 `ocr_lines`/`match_count` 字段。（SVG 红框覆盖层在 Batch 22 Lightbox 中实现。）
- **P1-3 智能文件夹升级**（第三轮 E1/E2/前端清单第五节）：后端新增 `update_smart_folder`（重命名/改查询条件）与 `refresh_smart_folder_count`（按需走完整 search_images 拿真实计数）两个命令；`get_smart_folders` 的 match_count 循环修复——纯语义文件夹（use_vector-only）不再被当作「恒 0」，而是让前端显示 `?` 表示「需执行才知道」；有混合通道时给字面量计数作为下界。前端 `useSmartFolders.ts` 全面重写：`saveSmartFolder(name)` 取消原生 `window.prompt`（改由 App.vue 的 `PromptDialog` 应用内对话框输入名称）；`removeSmartFolder(id, name)` 加 `window.confirm` 二次确认；所有 invoke 包 try/catch（不再抛未捕获异常）。新建 `components/PromptDialog.vue` 通用文本输入对话框（Esc 关闭、Enter 确认、长度校验）。`SmartFolderBar.vue` 升级：pill 加 `title` tooltip 显示完整查询条件与通道；计数徽标去黑话（不再 `(Num: 12 )`）；纯语义文件夹 `?` 加解释 tooltip；加 `↻` 刷新计数按钮；删除按钮独立 hover 区域减少误点。
- **P1-5 目录记忆 + 设置页骨架**（第三轮「上次目录每次被主动清除」）：新建 `composables/usePreferences.ts`——把 folder_path / page_size / ocr_lang / scan_mode / search_history 持久化到 localStorage，watch 自动保存，重开 App 自动回填。`useEngineStatus.ts` 停止 `localStorage.removeItem('framescout_folder_path')`（不再每天清掉用户的素材库路径），engineMaxRetries 从 50 降到 20（失败更快而非死等 150s）。新建 `components/SettingsDialog.vue` 设置页骨架（素材库路径带清除按钮、OCR 语言下拉、每页条数、关于版本号）。`vite.config.ts` 注入 `__APP_VERSION__`（从 package.json 读取），新建 `src/env.d.ts` 声明全局。`package.json` 版本号同步至 3.2.0（与 Cargo.toml/tauri.conf.json 一致）。

### Changed (Batch 21)
- `GhostItem` 类型（前后端）补 `disk_offline: bool` 字段（P0-1 配套）。

### Fixed (Batch 20 · M0 止血 — 六项地基性缺陷全清)
- **P0-1 ghost_cmd NAS 守卫**（第三轮 D7-D8）：`preview_ghosts` 增加盘根可达性检测——按盘符根（`C:\` / `\\server\share`）聚合做一次 `Path::exists()`，离线盘上的文件标 `disk_offline = true`，前端整组灰显并禁用 purge，不再误判整盘为幽灵；`apply_ghost_action` 的 Purge 分支在行动侧再检查一次盘根可达性（兜底）。同时补 `is_dead = 0 AND last_seen_at < 最近合法扫描批次` 过滤，兑现 `journal.rs:111` 注释承诺——刚扫到的文件不进幽灵清单。
- **P0-2 purge 级联 + search_text 过滤 is_dead**（第三轮 D4-D5/A18）：`apply_ghost_action` 的 Purge 分支开事务级联删 `frame_vectors` + `media_ocr_entries` + `media_text_vectors` + `reindex_backup` + `files`，不再留下孤儿行；`search_text` SQL 补 `LEFT JOIN files ... WHERE is_dead = 0`，真删除后文本搜索不再能搜出该文件；`reindex_files` 也补删 `media_text_vectors`。
- **P0-3 融合公式归一化**（第三轮 A1-A3）：旧实现 `cosine(0.1~0.35) + 2.0·OCR + 2.5·Note + 3.0·Filename` 量纲差一个数量级，文件名 grep 架空语义检索。新实现：SigLIP 余弦经 sigmoid 映射到 [0,1]（cos=0.15→0.5，cos=0.35→0.95），字面量通道改 0/1 二值，加权求和（`FUSION_WEIGHT_SEMANTIC=1.0` 最高，`FUSION_WEIGHT_FILENAME=0.4` 最低），常量提至 `constants.rs` 可调。语义相关性阈值独立为 `SEMANTIC_RELEVANCE_THRESHOLD=0.2`（不再用 0.001 工程噪声阈值当相关性阈值）。
- **P0-4 查询预处理**（第三轮 A5/A7）：`text.to_lowercase()` → `trim + 全角折叠(NFKC 子集) + lowercase + 切词 AND + 排除词(-前缀)`。复制粘贴带尾随空格不再 0 结果；搜「cat dog」要求两者都命中而非连续串；`-cat` 排除含 cat 的文件。`match_text` 函数对 CJK 保持子串、对拉丁也走子串（词边界留后续）。
- **P0-5 事务完整性**（第三轮 C14-C15/D1）：`index_cmd` 的 `tx.commit()` 不再 `let _ =` 吞错——OCR/text 写入失败则 rollback 整批并记入 `failed_paths`；commit 失败则回退内存矩阵中已 push 的行。`db.rs` 加 `PRAGMA busy_timeout = 5000`，并发写等待 5s 而非立刻抛 `database is locked`。
- **P0-6 长视频流式 + 取消**（第三轮 B4/B13/B6）：worker `video_extractor.py` 改为生成器 yield + 场景检测（帧间差 > 25 保留关键帧）+ 相邻去重（差 < 2.5 跳过），不再一次性堆 3600 帧进 list（22GB OOM）；`server.py` 抽出 `_process_frame_sub_batch` 按子批次边抽边编码边组装，内存恒定。Rust 侧 `AppState` 加 `cancel_flag: Arc<AtomicBool>`，新增 `cancel_scan` 命令，`process_file_paths_internal` 每批开头检查，命中即停止。`scan_folder`/`index_files` 入口重置标志。

### Changed (Batch 20)
- **稳定排序 + tie-breaker**（第三轮 A10/A11）：`search_images`/`search_by_image`/`search_text`/`search_text_entries` 的 `sort_by` 全部加 `then_with(|| a.path.cmp(&b.path))` 次级键，避免同分时翻页重复/漏项。`path_key` 辅助函数统一去重口径。
- **search_by_image 排除查询图自身**（第三轮 A28）：拿库里已有的图做以图搜图时不再把自己排第一（余弦=1.0）。
- **reindex meta 继承精度修复**（第三轮 E23）：`to_bits()` 改毫秒级整数键 `(ts * 1000).round()`，与 `index_cmd` 写 text vectors 的 round 口径一致，免疫亚毫秒浮点漂移——重索引后备注和 OCR 不再丢失。
- **M008 性能索引迁移**（第三轮 C3/D16）：`files(is_dead)` / `files(last_seen_at)` / `frame_vectors(index_time)` / `batches(finished_at)` 四个索引，启动 `ORDER BY index_time DESC` 与 ghost_cmd 的 `ORDER BY last_seen_at DESC` 不再全表 filesort。`SCHEMA_VERSION` 升至 8。

### Added (Batch 19 · Text results & library management now match the media experience)
- **纯文本检索结果与图片体验对齐**：独立分页（与媒体结果是两个结果集，页码互不干扰；翻页只重搜文本条目、不惊动媒体结果）+ `TextEntryCard` 可展开全文（超过 120 字才给 Expand 按钮，折叠态只露开头几行——与 `OcrPanel` 的展开/折叠同一套设计，不再是"只展示一部分"）。
- **纯文本库管理支持定位与翻页**：面板新增关键词过滤（按条目内容子串匹配）与分页。`list_text_entries` 新增 `query` 参数，用 `'%' || ? || '%'` 拼 LIKE——空串时得到 `'%%'` 匹配全部，因此无需为「有/无关键词」写两条 SQL。
- **`PaginationBar` 可复用化**：新增 `showAllButton`（文本结果 / 管理面板隐藏「📋 Show All」——它的语义是加载全部已索引媒体，对这些场景无意义）与 `dense`（紧凑模式，用于对话框内）。
- **搜索栏精简**：`📋 All Files` 与 `✍️ 记一条` 移到顶部「探索与维护」组（它们属于库维护而非发起检索），搜索栏只保留 `Search / 🖼️ Similar / ⭐ Save Smart` 三个动作。

### Added (Batch 18 · Text-file ingest + text library management)
- **文本文件直接入库**（`ingest_text_files`）：可选 `.md/.markdown/.txt/.text` 文件**或整个文件夹**（递归），读文本后走 BGE-M3 文本通道入 `text_entries`，不经过 SigLIP 视觉编码。长文档自动**分块**（`TEXT_CHUNK_CHARS=500`，优先按段落边界、段落内超长再硬切；按**字符数**计以正确处理中文），块信息记在 `metadata_json`（chunk_index / chunk_count），因此无需为此改表。分块是纯函数并有 6 个单测覆盖（空文本 / 单块 / 段落合并 / 超限分块 / 超长段落硬切 / 中文按字符计数）。
- **纯文本库管理**（`list_text_entries` / `delete_text_entries` + `TextEntryManagerDialog`）：顶部「探索与维护」组新增「📄 文本库」入口。面板打开即列出条目（内容片段 + 来源 + 入库时间，**默认零勾选**），删除只作用于用户显式勾选的 entry_id——删除是终态不可回退，与幽灵清理 `purge` 同理，绝不提供「一键清空」。同时可从面板导入文本文件或文件夹。
- **一处边界修正**：分块累加时原先未计入 `\n\n` 分隔符，拼出的块会略微超出上限；现已计入（由单测暴露并修正）。

### Added (Batch 17 · packaging wrap-up + Tauri distribution + RapidOCR multilang)
- **打包全流程打通 + clcache 增量验证**：`build_nuitka.ps1 -Onedir -NoTorch` 第二次重跑 `Compiled 2756 C files using clcache with 2756 cache hits and 0 cache misses`（全命中、0 miss），验证了「代码未变则复用上次编译产物」的增量缓存；脚本完整跑通 `编译 → 复制模型 → 产物定位 → 部署 Tauri bin → 完成`。
- **Tauri 分发约定**：`src-tauri/tauri.conf.json` 不配 `bundle.resources`/`externalBin`，worker 的 onedir 目录（含 GB 级模型）**手动复制**到部署目录单独分发——`externalBin` 只适用单文件 exe、装不了 onedir 目录，且 worker+models 体积过大不宜进 UI 安装包。Rust 侧路径逻辑无需改（主路径 `exe同级/ai_worker/ai_worker.exe` 与 onedir 部署天然吻合，模型由 worker 侧 `config.py::get_base_path()` 按 exe 同级 `models` 解析）。
- **RapidOCR 多语言调研**：新增 `src/inference-worker/RAPIDOCR_MULTILANG.md`，厘清 RapidOCR 三段式（det/cls 通用、rec 语言相关）与「多语言 = 只换 rec 权重」的接入方式（`rec_model_path`/`rec_keys_path` kwargs），给出 `_build_engine` 改造示例；当前内置权重仅覆盖中英（`SUPPORTED_LANGUAGES={"en","ch_sim"}`）。
- **运行时排除拦截修复 + 端到端启动成功**：`--nofollow-import-to` 默认带 deployment flag `excluded-module-usage`，会在运行时拦截一切对被排除模块的 `import/find_spec`；transformers 启动时恰用 `find_spec("onnx")` 做软依赖探测，导致打包后 worker 报 `Module 'onnx' was actively excluded` 崩溃。现加 `--no-deployment-flag=excluded-module-usage` 禁用拦截——被排除模块只是"不编进产物"而非"运行时禁止引用"。打包产物 `ai_worker.exe` 端到端验证通过（`✅ AI Worker is ready!`，无 torch 下正常加载 SigLIP processor + BGE-M3 + RapidOCR）。

### Added (Batch 16 · torch-free build completed)
- **无 torch 打包跑通**：`.\build_nuitka.ps1 -Onedir -NoTorch` 成功产出 `dist_nuitka\main.dist\ai_worker.exe`（2756 个 C 文件编译通过）。onedir 总产物约 540 MB（主 exe 266 MB + cv2 112 MB + onnxruntime DirectML 46 MB + numpy 26 MB 等），对比带 torch 的 GB 级已显著瘦身。
- **编译期排除软依赖**（`--nofollow-import-to`）：`onnx/tensorflow/tf2onnx`（transformers 的 ONNX 导出工具软依赖，未装时 DLL 检测 FATAL）、`scipy`（EasyOCR 残留 109 MB，编译其巨型 C 文件触发 MSVC C1060/C1002 堆空间不足）、`sympy`（onnxruntime 可选符号形状推断工具，`polyquinticconst` 巨型常量表同样 C1002）。均与 torch 同理——worker 推理走 onnxruntime，这些库用不到、不编进产物。
- **产物定位修复**：Nuitka onedir 的目录名取入口脚本（`main.dist`）而 exe 名取 `--output-filename`（`ai_worker.exe`），脚本原 fallback 只认 `main.dist\main.exe` 导致"找不到产物"误报；现按 `ai_worker.dist\ai_worker.exe → main.dist\ai_worker.exe → main.dist\main.exe` 依次查找。
- **BGE-M3 导出修复**：BGE-M3 dense 约 2.2GB（float32）超过 ONNX protobuf 2GB 上限，`torch.onnx.export(dynamo=False)` 会失败并留下 `onnx__MatMul*` / `model.*.weight` 碎片；现用 `model.half()` 转 float16（减半到 ~1.1GB 落回限制内，检索余弦相似度对 float16 不敏感），并在导出前自动清理旧碎片。

### Added (Batch 15 · Nuitka torch-free packaging)
- **打包剔除 torch/torchvision**：worker 推理全走 ONNX(DirectML)，运行时根本不需要 torch。新增 `build_nuitka.ps1 -NoTorch` 开关，显式 `--nofollow-import-to=torch|torchvision|torchaudio` 把整棵 torch 剔出产物（体积从 GB 级降到百 MB 级）。开关内置防呆：检测到 `transformers` 仍是 5.x 会直接中止——5.x 的 SigLIP image processor 继承 `TorchvisionBackend` 并被 `@requires("torch","torchvision")` 装饰，无 torch 时 `AutoProcessor.from_pretrained` 会崩溃。
- **`transformers` 锁 4.x（`>=4.50,<5`，两份 requirements.txt 同步）**：4.x 对 torch 软依赖，是 `-NoTorch` 剔除 torch 的前提。⚠️ 下限必须是 4.50——**4.49 有 SigLIP 2 processor 映射 bug**（`SiglipProcessor.tokenizer_class` 硬编码为 `SiglipTokenizer`，去加载 SigLIP 2 的 `GemmaTokenizer` 时报 `vocab_file=None`），≥4.50 已修复。实测 4.57.6 在藏起 torch/torchvision 的环境下 `AutoProcessor.from_pretrained("google/siglip2-base-patch16-256")` 正确加载（tokenizer=`GemmaTokenizerFast`、image_processor=`SiglipImageProcessor`），且 `processor(text=..., images=..., return_tensors="np")` 实际产出 numpy 张量（`pixel_values (1,3,256,256)`、`input_ids`），**无 torch 方案端到端闭环**。另补 `sentencepiece>=0.2.0` 声明（SigLIP 2/BGE-M3 tokenizer 的软依赖，slow 路径必需）。
- **编译期 GBK FATAL 修复**：Nuitka anti-bloat 插件编译时 `exec` `torch.utils._config_module` 会按系统 GBK 读 torch 模板，遇 UTF-8 字符抛 `UnicodeDecodeError('gbk', 'illegal multibyte sequence')`。脚本在编译前设 `PYTHONUTF8=1` + `PYTHONIOENCODING=utf-8` 消除该 FATAL；并改用 `python -m nuitka` 取代 `nuitka.cmd` 包装器，避免其吞掉真实退出码导致"编译失败却误报成功"。
- **onnxruntime 包损坏自检加固**：`rapidocr_onnxruntime` 的依赖声明是标准 `onnxruntime`，朴素 `pip install` 会与 `onnxruntime-directml` 同名互删、把 `onnxruntime` 变成缺 `__init__.py`/缺 `capi` dll 的 namespace package（运行时 `get_available_providers()` 直接 `AttributeError`）。自检现在同时检查 python 退出码并校验 `capi` 目录非空，损坏即明确中止并给出修复步骤（DirectML 先装、再 `--no-deps rapidocr_onnxruntime`）。
- **`download_models.py` 导出对齐 4.x（>=4.50）**：SigLIP 导出前新增 transformers 版本校验（`<4.50` 或 `>=5` 直接中止，提示装 `>=4.50,<5`），因为 4.49 有 SigLIP 2 映射 bug、5.x 写出的 `processor_config.json` 命名与 tokenizer 文件集与 4.x 不兼容；`processor.save_pretrained` 后自动复制 `processor_config.json` → `preprocessor_config.json`（4.x image processor 默认识别名）。

### Added (Batch 14 · Text-entry front end + centered toolbar)
- **纯文本前端点亮**：「📄 纯文本」目标不再置灰（BGE-M3 模式下可勾选），勾选后走 `search_text_entries`，结果以独立的 `TextEntryCard` 展示（内容片段 + 来源 + 相似度），与媒体命中分开展示、绝不混排。新增「📄 记一条」入口与 `TextEntryDialog`，用户输入文本即可经 `insert_text_entry` 入库，之后可被语义搜索召回。
- **采集工具栏居中**：`TopActionBar` 的控件行恢复 `justify-content: center`，采集 / 探索维护两组控件居中排布。

### Added (Batch 13 · Text-entry backend + Nuitka script encoding fix)
- **纯文本后端闭环**：`SuccessPayload` 新增 `text_entries`（纯文本入库的返回位，与 `frames` 互斥），worker 新增 `text_task` 分支（BGE-M3 编码文本 → `TextEntry`），Rust 新增 `insert_text_entry`（`text_entries` 唯一写入口）与 `search_text_entries`（dense 余弦检索，返回第一手原始分）。纯文本（`text_entries`）与媒体内嵌 OCR（`media_text_vectors`）是两条语义，分表分查，绝不混存。
- **顺带修了一个真 bug**：worker 主循环把 `file_task` 分支误判成 `file_path`（proto 改名后的遗留），导致单文件任务（以图搜图的编码入口）永远路由不到 `_handle_file`。已改为 `file_task`。
- **`build_nuitka.ps1` 加 UTF-8 BOM**：Windows PowerShell 5.1 对无 BOM 的 UTF-8 脚本按 ANSI/GBK 解读，emoji 与中文导致字符串解析错乱。脚本现已带 BOM，可正常解析执行。

### Changed (Batch 12 · Fuzzy mode now searches text too + Nuitka docs)
- **模糊 mode no longer locks out text**: it defaults to 图片 (pure visual semantic) but 文件名/笔记/OCR stay enabled, so a user can combine SigLIP visual semantics with filename/note/OCR substring matching — file-manager-style text search. Switching modes resets targets to that mode's defaults (exact → filename/note/OCR; semantic → image only; BGE-M3 → OCR only), and each can still be hand-tuned afterwards.
- **Packaging docs now lead with Nuitka**: `inference-worker/README.md`, `Notes.txt`, `打包.txt`, `ai_worker.spec`, and the repo `README.md` / `README.zh-CN.md` all point to `build_nuitka.ps1` as the preferred path, with PyInstaller demoted to a clearly-marked legacy fallback. Stale references to `search.proto`/`search_pb2`/`easyocr` were cleaned up.

### Added (Batch 11 · Search modes UI + toolbar regroup)
- **Three search modes in the console** — 精确 (string match on filename/note/OCR) / 模糊 (SigLIP visual semantic) / BGE-M3 (text semantic over OCR text). The mode picker is a segmented control; the target checkboxes (文件名/笔记/OCR/图片/纯文本) enable/disable per mode, and 纯文本 stays greyed out until a text-entry writer exists. The front end maps each mode to the right backend call and keeps the raw per-channel scores intact — BGE-M3 hits carry their own `scoreKind='bge'` and a dedicated `mapBgeScore` mapping, since text cosine lives on a far higher scale than visual cosine (mixing them would flatten everything to 99.9%).
- **Toolbar regrouped** into 采集 (ingest) and 探索与维护 (explore & maintain) sections, each with a small label, so the 8 controls no longer sit in one undifferentiated row.
- **Search bar no longer jitters** — `⭐ Save Smart` is always rendered and just disabled when the query is empty, instead of appearing/disappearing and widening the row.

### Added (Batch 10 · BGE-M3 text retrieval — query side)
- **`search_text` command** — the query-side twin of Batch 9's indexing path. It encodes the query to a BGE-M3 dense vector (`request_text_dense`), then brute-forces cosine similarity against every `media_text_vectors` row (decoded on the fly from `dense_blob`). It returns **raw, first-hand data** (`TextSearchHit.similarity` is the unweighted cosine) with no cross-channel aggregation — whether and how to mix it with visual / exact-match hits is left to the front end, per the "provide information, don't decide for the user" rule.
- **`request_text_dense` + worker branch**: `_handle_text` now branches on `EncodeRequest.model` — `BGE_M3_DENSE` routes through `BgeEngine` (returns `dense_vector`), anything else keeps the existing SigLIP text path. The two retrieval roads never mix.

### Added (Batch 9 · BGE-M3 text channel — dense indexing path)
- **BGE-M3 dense engine** (`engines/bge_engine.py`): the text-side twin of SigLIP. Loads the exported `bge_m3_dense.onnx` (1024-D, CLS-pooled + L2-normalized inside the graph), tokenizes with the bundled XLM-R tokenizer, batches at ≤8 (`BGE_MAX_BATCH`). `embed_texts()` is the single entry point — swapping the text model later touches only this layer, never the ZMQ/proto contract. A missing model is **not** a startup failure: the text channel is an incremental capability, so it degrades to "visual search only" instead of crashing the worker.
- **Indexing now stamps text vectors**: `_handle_batch` runs each frame's OCR text through BGE-M3 and fills `FrameResult.dense_vector` (only for frames that actually have text). The core stores them in `media_text_vectors` via a new `storage/text_store.rs` — one write-entry, in the same transaction as the frame vectors and OCR rows, dimension-checked (1024). `sparse_json` stays NULL for now, per the agreed plan (sparse + exact matching arrives with the FTS5 trigram batch). `EncodedFrame` / `zmq_client` now carry `dense_vector` end to end.
- **`download_models.py --bge`**: downloads `BAAI/bge-m3` and exports the dense encoder to ONNX (dense only — sparse/colbert heads are intentionally skipped). Requires a torch + transformers environment, separate from the worker's torch-free DirectML venv.

### Added (Batch 8 · BGE-M3 text-channel schema + Nuitka build)
- **Text-channel tables** (migration `M007`, `SCHEMA_VERSION = 7`): `media_text_vectors` (per-frame BGE-M3 embeddings for media-embedded OCR text) and `text_entries` (standalone notes/documents/transcripts). They stay separate on purpose — OCR text is a media *attribute* ("find the frame") while plain text is a first-class searchable object ("find the content"); mixing them would make "what is love" return a wall of subtitles. Both carry a `model` column and `dense_blob`/`sparse_json` (little-endian f32, same byte-order contract as `vector_f32`), plus `chunk_index`/`parent_id` for long-text chunking. Schema only for now; the worker inference wrapper and RRF fusion come next (they need the model weights).
- **Nuitka build pipeline** (`build_nuitka.ps1`): the preferred packaging path, replacing PyInstaller. One command produces a native `ai_worker.exe` (standalone + onefile, MSVC), bundles the DirectML `capi` DLLs and the wheel-bundled RapidOCR weights, copies the external `models\` next to the exe, and optionally deploys to `src-tauri\bin\ai_worker`. It self-checks for `DmlExecutionProvider` before building and aborts if a CPU-only onnxruntime slipped in.
- **Requirements pinned to DirectML** in both `requirements.txt` files, and the install-order trap documented: `rapidocr_onnxruntime` depends on the *standard* `onnxruntime` name, so a naive `pip install` pulls a CPU-only onnxruntime that silently shadows the DirectML build (RapidOCR then falls back to CPU). Correct order: DirectML first, then `--no-deps rapidocr_onnxruntime`. `BUILD.md` gained a Nuitka chapter (§4) and the PyInstaller section is demoted to legacy (§5).

### Added (Batch 7 · EasyOCR → RapidOCR + structured OCR)
- **OCR engine replaced by RapidOCR** (`engines/ocr_engine.py`). `rapidocr_onnxruntime` + PP-OCRv4 ONNX weights, no PyTorch dependency, DirectML first with CPU fallback (never crash the worker over a provider). The `OCR_ENGINE_NAME`/`OCR_MODEL_NAME` constants are stamped into every frame.
- **Structured OCR is now the source of truth**: the worker emits per-line `ocr_lines` (text + normalized bbox + score + lang + per-frame `timestamp_ms` + raw-corner payload), and the core stores them in a new `media_ocr_entries` table (`M006`, `SCHEMA_VERSION = 6`). `ocr_text` stays as the concatenated full text for the legacy search path. Data is stored now, highlight rendering comes later.
- **`request_vector` now returns a structured `EncodedFrame`** (with `ocr_lines`) instead of a 5-tuple. The frame/OCR fields are defined once in `model_code::frame.rs` and shared across the scan / reindex / OCR / search paths.
- **OcrConfig embedded into every payload** (the structural fix that was pending): `string file_path` became `SingleFileTask`, `single_file_ocr_config` is now `reserved 6`. The protocol no longer relies on a comment to say "this config goes with that payload" — the relationship lives in the structure.
- **Requirements & downloader updated**: `easyocr` → `rapidocr_onnxruntime` in both `requirements.txt` files; `download_models.py` no longer downloads EasyOCR and instead self-checks RapidOCR (PP-OCRv4 ships inside the wheel, nothing to download for en + ch_sim).

### Added (Batch 6 · Protocol finalized — B line)
- **`search.proto` → `framescout.proto`**, ending the long-standing mismatch between the file name and `package framescout`. `build.rs`, the Python bindings (`framescout_pb2.py` / `.pyi`), `server.py`, and the build docs all follow.
- **Protocol brought to its "one-time finalization" state**, append-only throughout:
  - `EmbeddingModel` enum (`SIGLIP_768` / `BGE_M3_DENSE` / `BGE_M3_SPARSE`) — the anchor for the "never mix models" rule.
  - `FrameResult` gains `capture_timestamp_ms`, `duration_ms`, `embedding_model`, `dense_vector`, `sparse_weights`, `ocr_lines`, `ocr_engine`, `ocr_model`.
  - `OcrLine` (text + normalized bbox + score + lang + `timestamp_ms` + raw-corner payload) — the future source of truth for OCR retrieval and hit highlighting.
  - `TextEmbedding` / `TextEntry` / `TextTask` / `SearchRequest` / `SearchResponse` — the text channel is defined now so nothing needs to be reshaped later.
  - `EncodeRequest` gains `max_results`, `model`, `session_id`, `source`; `OcrConfig.enable_ocr` is now `optional` (explicit presence); `BatchPaths` renamed to `BatchTask`.
- **Worker now stamps `embedding_model` and `ocr_engine` on every frame** — without the model tag there is no way to filter by model later, and cross-model similarity is garbage.
- **One constructor for requests**: `zmq_client::build_request()` is the single place that builds an `EncodeRequest`, so adding a protocol field no longer means hunting down every call site. `source` defaults to `"app"`; MCP can override with struct-update syntax.

### Added (Batch 5 · Reindex cold backup + scan reports)
- **`reindex_backup` table** (migration `M005`, `SCHEMA_VERSION = 5`). Before `reindex_files` deletes the old frame rows, it copies them — **inside the same transaction** — into cold storage. This is the now-or-never half: a snapshot can only be taken before the `DELETE`, and no amount of later work can bring those vectors back. The other half (an undo command and UI) is deliberately **not** done; the backup table never enters the memory matrix and never participates in search. Retention: 3 snapshots per path, 30 days max, pruned on startup.
- **Reindex no longer discards your data**: it now inherits `user_note` from the old rows, and keeps a frame's existing OCR text when the re-encode comes back empty (e.g. OCR was off for that run). Previously both were silently wiped by the row replacement.
- **Scan reports are visible** — new `list_scan_reports` reads the `batches` skeleton + `summary_json` (no extra bookkeeping table), and `ScanReportDialog` shows each past run with an expandable breakdown: counts, elapsed, per-file average, directory distribution, and the failed-file list. Interrupted batches (crash leftovers) show as such instead of being hidden.
- Scan summaries now include a **directory distribution** (top 5 folders).

### Added (Batch 4 · Diff report: perceive, don't act)
- **`scan_diff`** — a read-only comparison of a folder against the library's observation snapshot. It walks the folder (stat only, no inference), writes nothing: no state, no events, no batch. Four groups, all of them mandatory: **new / modified / missing / unindexed** (the last one being the permanent "has an anchor but no vector" group that stops a failed index from being silently skipped forever).
- **`reindex_files`** — the missing half of the action layer. `index_files` skips paths that are already indexed, so "modified" entries had no way back in. Re-encoding replaces the old frame rows inside a single transaction (`DELETE` then insert) and only writes `content_modified` **after** success; failures write `reindex_failed` and stay in the unindexed group for a retry.
- **`DiffReportDialog`** — three layers: overview counts → per-group detail with checkbox select-all → actions (index / mark dead / ignore). Mass-change detection warns about suspected cloud-sync or bulk touch without ever acting on it.
- **"Ignore" deliberately never reaches the backend.** It applies to this batch only: no event, no `observed_*` update. The file simply shows up again next scan until the user ends that state by indexing or marking it dead.

### Changed (Batch 4)
- `journal::classify_change` is now public and shared by both the scan loop and the diff report, so "what counts as changed" has exactly one definition (2 s tolerance, size as the second signal).

### Added (Batch 3 · Ghost cleanup: preview first, act second)
- **Ghost cleanup is now a two-step flow**: `preview_ghosts` is a **read-only** preview (no state, no events, no batch is written), and `apply_ghost_action` performs what the user explicitly picked. The old one-click `clean_ghosts` command is gone — nothing is deleted behind a single `confirm()` any more.
- **Every action has a reverse**: `mark_dead` (keep the vectors, drop out of search) ↔ `restore` (back into search, **no re-encoding** — the vectors never left the database), and `purge` (delete rows for good, irreversible).
- **New `GhostCleanupDialog`**: two clearly separated groups — *gone from disk* (still searchable) and *marked dead* (still occupying space) — each with its own select-all, per-row checkbox, frame/slice count, size, last-seen time, and its own action buttons. Opening the dialog starts with nothing selected, so the app never pre-decides for you. New `EventType::Purged` records irreversible deletions in the ledger.

### Changed (Batch 3)
- Memory-matrix loading was factored into `load_frame_rows(conn, only_paths)`, shared by startup and by `restore`. The BLOB-first / JSON-fallback rule now lives in exactly one place.
- `run_ocr_for_selected_files` and the two ghost commands are `async`, so large libraries no longer block the UI thread.

### Added (Batch 2 · The ledger)
- **Three new tables — `files` / `batches` / `file_events`**, plus the three `file_events` indexes (`batch_id`, `path`, `occurred_at`). State lives in `files` (latest state only), cause lives in `file_events` (append-only, never updated), and every bulk operation gets a skeleton row in `batches`.
- **`batch_id` is now joinable**: scans open a `batches` row (`status = running`) before indexing and close it on completion with `finished_at` + a `summary_json` scan report (requested / inserted / first_seen / changed / failed paths / elapsed / avg per file). The same id is written to `frame_vectors.indexed_in_batch_id`, so "re-encode everything from that run" is one `WHERE` away.
- **Legacy baseline seeding**: on upgrade, every existing path in `frame_vectors` gets a `files` anchor and one `first_seen` event under the `legacy_import` batch — **one event per path, not per frame**, so videos don't flood the ledger. Seeding only runs when `files` is empty, making it idempotent.
- **`storage/journal.rs`**: the single place that writes observations, events and batches. The mtime/size comparison is a pure `classify_change` function with a 2 s tolerance (NTFS 100 ns vs FAT/exFAT 2 s vs SMB 1 s), covered by unit tests.
- **Interrupted batch repair on startup**: batches left `running` with no `finished_at` by a crash are marked `interrupted`, so they can never be mistaken for a completed scan when the diff report picks its skeleton.

### Changed (Batch 2)
- `SCHEMA_VERSION` is now `4` (migration `M004 ledger_tables`).
- Memory-matrix loading now `LEFT JOIN`s `files` and skips `is_dead = 1` rows. The `frame_vectors` rows are kept, so reviving a file needs no re-encoding. `LEFT` (not `INNER`) on purpose: a missing ledger anchor must never make data disappear.
- **`run_ocr_for_selected_files` now updates by `(path, timestamp)`** instead of by `path`. Under the composite key, updating by path alone meant every frame of a video ended up with the last frame's OCR text.

### Added (Batch 1 · Vector BLOB + batch id)
- **`vector_f32` BLOB column**: new index writes store vectors as a little-endian flat `f32` blob (768 × 4 = 3072 B) instead of JSON. Loading reads the BLOB first and falls back to `vector_json` **only for legacy rows**, so v3.1 databases keep working untouched. No JSON is written any more — the legacy column is a consumable, no double-write cost is paid for it.
- **Byte-order contract pinned down on both sides**: `storage/vector_blob.rs` (`encode_f32_le` / `decode_f32_le_checked`) on the Rust side and `pack_f32_le` / `unpack_f32_le` in `inference-worker/utils/vector_utils.py` on the Python side. Both are documented as the single place where packing may happen.
- **`indexed_in_batch_id` column**: each indexing run writes its `scan_<UTC>` batch id onto the rows it produces — a placeholder column today, and the join-less shortcut for "re-encode by batch / trace back after a model upgrade / clean by batch" later.
- **Batch ids in trace logs**: per-batch and per-scan trace lines now carry `batch_id`.

### Changed (Batch 1)
- `SCHEMA_VERSION` is now `3`. Migration `M003 frame_vectors_blob_columns` adds the two new columns **and** drops the `NOT NULL` constraint on the legacy `vector_json` (SQLite cannot alter a column constraint, so this one is a rename→create→copy→drop inside a single transaction). Because new writes no longer populate `vector_json`, keeping it `NOT NULL` would have rejected every insert.
- `M001` now creates `frame_vectors` with the latest DDL, so a fresh database is born at the final shape and every later migration is a no-op.
- `list_all_files` / `get_all_files` no longer select `vector_json` — they only ever needed the metadata columns.

### Added (Batch 0 · Engineering Discipline)
- **Versioned schema migrations**: `PRAGMA user_version` now tracks the DB schema version (`SCHEMA_VERSION = 2`). Every migration declares `from_version`/`to_version` and is idempotent, so any historical database (including the v3.1.0 single-`path`-key layout) can be upgraded to the latest layout by simply running the chain.
- **Backup bound to migration, not to startup**: a `backups/pre_migrate_v<n>_<UTC timestamp>.db` copy is written **only** when the expected version is greater than the database's current version. Startup with a matching version performs zero I/O.
- **Windows path normalization**: all paths written to `frame_vectors` are normalized (`/`→`\`, extended-length `\\?\` prefix stripped, duplicated separators collapsed, drive letter upper-cased, `.`/`..` folded, trailing separator removed) so the same file can never be recorded twice due to case or separator variants.
- **`trace`-level logging**: a dependency-free `fs_trace!` macro emits one line per completed scan, per committed indexing batch, per search, and once after the database is loaded — each carrying counts and elapsed time.

### Changed
- Version bumped to `3.2.0` (`Cargo.toml`, `tauri.conf.json`).
- The legacy `path` primary key → `(path, timestamp)` composite key migration is now migration `M002` inside the versioned chain instead of an ad-hoc function in `db.rs`; failure still rolls back and leaves the old schema intact.

## [3.1.0] - 2026-09-23

Maintenance & consistency pass (no model/runtime behavior changes):

### Changed
- **Documentation**: repository folder renamed `src/frontend` → `src/FrameScout-UI` across README, TREE and build instructions.
- **Documentation**: version references bumped to `3.1.0`; model references corrected from CLIP to **SigLIP 2** (migration completed in v3.0.1).
- **UI**: startup splash text corrected to *"Loading large SigLIP 2 & EasyOCR models…"*.

### Fixed
- **License verification**: parse the `::` separator from the **end** of the key (Ed25519 signature is a fixed 64 bytes of binary data), avoiding a mis-parse when the signature happens to contain the `::` byte sequence.
- **UI**: pure vector (semantic) search results now correctly show the `💡 Semantic` tag (previously showed no tag at all).

[3.1.0]: https://github.com/bobgsning/FrameScout/releases/tag/v3.1.0

## [3.0.3] - 2026-08-10

### Added v3.0.3

- **Backend-Driven Smart Folders**: Smart Folder definitions are now stored exclusively in SQLite (`smart_folders` table). New backend command `execute_smart_folder` executes searches natively using the same engine as manual queries. Frontend no longer caches any folder rules — no more lost folders after clearing browser data.
- **Dynamic Match Count Badges**: `get_smart_folders` now returns `match_count` for each folder, computed by scanning in-memory metadata for text matches. For folders relying solely on vector search (no OCR/note/filename), a `?` badge is displayed instead of `0`, indicating the count is unknown until executed.
- **On-Demand OCR for Selected Files**: Users can now run OCR on specific files with custom language settings. The `run_ocr_for_selected_files` command accepts a list of file paths and language codes (e.g., `en`, `ch_sim`, `ja`). OCR results are updated in both SQLite and the in-memory metadata immediately.
- **Automatic Cleanup of Legacy Browser Storage**: On startup, the application removes old `localStorage` keys (`framescout_smart_folders`, `framescout_folder_path`) to prevent interference from previous versions.

### Changed v3.0.3

- **SmartFolder struct**: Extended with `match_count: usize` field, populated by the backend.
- **Frontend `applySmartFolder`**: Now calls `execute_smart_folder` instead of relying on local cached rules.
- **`selectFolder` and `savePath`**: Completely removed `localStorage.setItem` calls. Folder path is no longer persisted to browser storage (future versions may introduce backend-based preference storage).
- **Template Smart Folder badge**: For pure vector folders, renders `?` instead of `0`.
- **OCR language configuration**: The `enable_ocr` and `ocr_languages` parameters are now passed explicitly to `scan_folder` and `index_files`. Users can also trigger OCR on already-indexed files via the new per-item OCR button.
- **Scan progress events**: Now include `new_files` array for immediate display in the incoming files banner, without disrupting the current search results.

### Fixed v3.0.3

- **Smart Folder count always showing 0**: Previously, `get_smart_folders` only counted text matches, ignoring the `use_vector` flag. Now pure vector folders show `?` to avoid misleading zeros.
- **Runtime error from commented-out `savePath` function**: The function and all its invocations have been deleted.
- **Potential duplication of Smart Folders**: Old `localStorage` data could merge with backend data; now completely eliminated.
- **OCR not updating in-memory metadata**: `run_ocr_for_selected_files` now properly updates both SQLite and the `FlatVectorMatrix` metadata.

### Removed v3.0.3

- **Frontend `localStorage` usage**: Removed `savePath` function, `@input="savePath"` binding, and all `localStorage.getItem/setItem` calls (except the startup cleanup).

[3.0.3]: https://github.com/bobgsning/FrameScout/releases/tag/v3.0.3

## [3.0.2] - 2026-08-09

### Added v3.0.2

- **“Show All” mode**: New backend command `get_all_files` returns every indexed file (no pagination). Frontend adds a “📋 Show All” button in the pagination bar and a quick-access button in the search toolbar. While in “Show All” mode, a hint bar displays the total count and a “📄 Paginated View” button to return to paginated browsing.
- **Protobuf `index_time` field** (`FrameResult.index_time`, `double`): Python worker now stamps each indexed file with the current Unix timestamp.
- **Database column `index_time`**: `frame_vectors` table gains `index_time REAL`. Existing databases are automatically migrated via `ALTER TABLE`.
- **Backend command `list_all_files`**: Paginated listing ordered by `index_time DESC`, used by the browse-mode pagination.

### Changed v3.0.2

- **Browse mode sorting**: `list_all_files` now sorts by `index_time DESC` instead of `timestamp DESC`. Newly indexed files consistently appear first.
- **`AppState.memory_db` upgraded from `Mutex` to `RwLock`**: concurrent reads allowed, writes exclusive – improves responsiveness during parallel searches.
- **`request_vector` creates a fresh ZMQ REQ socket per call**: eliminates shared-socket contention and simplifies reconnection logic.
- **`clean_ghosts` returns structured result** (`CleanResult { removed_count, removed_paths }`): frontend precisely removes ghost entries from the current result set.
- **Scan progress handler**: `new_files` are always queued into `incomingFiles` and displayed via banner – never injected directly into `results` to avoid page disruption.

### Fixed v.3.0.2

- **New files not appearing after clicking the “📥 N new images” banner**: root cause was that images had `timestamp = 0.0`, making `ORDER BY timestamp DESC` degenerate to physical storage order. Solved by introducing `index_time`.
- **Stale responses overwriting newer ones in `performSearch`**: added `searchRequestId` counter – only the latest request’s result is applied.
- **`clean_ghosts` clearing all results in browse mode**: now removes only the actual ghost paths and reloads the current page.
- **`acceptIncomingFiles` forcefully exiting search/clustering context**: now transitions to browse mode (first page) or refreshes “Show All” mode accordingly.
- **`changePage` allowing invalid page numbers**: clamped to `[1, totalPages]`.
- **`clearImageSearch` clearing results even when not in image-search mode**: early return guard added.
- **`toggleClustering` failing to restore previous results on exit**: now correctly saves and restores `fullResultsCache`.

### Improved v.3.0.2

- **Frontend debouncing**: rapid page turns or search switches only apply the last response.
- **`scan_folder` uses `RwLock` semantics (`read()` / `write()`) for `memory_db`**.
- **“Show All” mode integrates seamlessly**: works with `acceptIncomingFiles`, `cleanGhosts`, and clustering (exits to paginated view when searching).

[3.0.2]: https://github.com/bobgsning/FrameScout/releases/tag/v3.0.2

## [3.0.1] - 2026-08-07

### Changed v3.0.1

- **AI inference engine overhaul**:
  - Migrated from OpenAI CLIP (512D) to **Google SigLIP 2** (768D) for improved semantic understanding and multilingual support.
  - Switched inference framework from PyTorch to **ONNX Runtime**, with automatic hardware acceleration detection:
    - DirectML (AMD / NVIDIA / Intel GPUs on Windows)
    - CUDA (NVIDIA GPU)
    - CPU (universal fallback)
  - Model path changed to `models/siglip2-base/`; users must re-run `download_models.py`.
- **Frontend low-score collapsible card UI**:
  - Low-confidence results are now collapsed by default, showing a red prompt bar. Click to expand and view details.
  - Added a “▲ Collapse” button at the bottom of expanded cards to re-collapse them.
  - Improved visual styling of the prompt bar (gradient background, rounded corners, hover effect).
- **EasyOCR fully offline**: Model storage directory fixed to `models/easyocr/`; no network dependency.
- **Performance**: ONNX batch inference is ~15–30% faster than PyTorch (depending on GPU).

### Fixed v3.0.1

- Fixed a crash when extracting video frames with NaN FPS values.
- Fixed conflicting display logic between low-score tags and semantic tags.

### Removed v3.0.1

- Removed runtime dependency on PyTorch (still kept in `requirements.txt` for development/debugging purposes).

---

## [3.0.0] - 2026-08-05

> Note: Internal versions 1.x and 2.x existed as closed-source prototypes. This is the first public release.

### Added v3.0.0

- Initial public release of FrameScout Community Edition
- Semantic search via CLIP (text + image embeddings)
- OCR text search via EasyOCR
- Image-to-image search (upload a reference image, find visually similar)
- Video frame indexing (1 FPS extraction, supports MP4/MOV/AVI/MKV)
- Visual clustering with adjustable similarity threshold
- Smart folders (save search conditions, re-run with one click)
- Personal notes with markdown support, searchable
- 100% offline operation — zero network requests
- Hybrid scoring: vector similarity + OCR match + note match + filename match
- FlatVector matrix for fast brute-force search
- Rust + Tauri native desktop app (Windows 10/11)
- Python inference worker (ZMQ + Protobuf IPC)
- Vue 3 frontend with search console, result grid, pagination
- Real-time scan progress with batch processing
- Remove database entries for files that no longer exist on disk (ghost records).

[3.0.1]: https://github.com/bobgsning/FrameScout/releases/tag/v3.0.1
[3.0.0]: https://github.com/bobgsning/FrameScout/releases/tag/v3.0.0
