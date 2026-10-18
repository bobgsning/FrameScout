# Changelog

All notable changes to this project will be documented in this file.

## [3.3.0] - 2026-10-18

本版主题是「**逻辑搜索重构 + 数据安全闭环**」：智能文件夹与搜索栏统一为多条件行求值
（关键词 / 时间 / 目录 / 批次 / 文件类型 / 基准图，且 / 或 / 排除 / 逐条件阈值），
补齐数据库备份的「出」（应用内恢复 + 预览 + 恢复前自动备份）、来源批次与入库时间范围分区搜索、
扫描暂停 / 继续、自动清理策略（给开关并告知）；后端全面改用稳定错误码，文案交予前端 9 语 i18n；
另以 LGPL 版 FFmpeg 兜底解码 MKV/AVI，并接上视频「切片段」。界面语言自本版起覆盖 9 种。

### Nuitka onedir 模型路径解析回归修复（发布前核查）

**问题**：onedir 打包的 worker 启动即报
`SigLIP 2 ONNX models missing in …\ai_worker\..\models\siglip2-base`（多了一层父目录），
同时 `ocr_manifest.json not found`——模型、manifest、ffmpeg 明明都与 exe 同级。溯源发现是
Batch 53 给 `config.py::get_base_path()` 优先返回 `__compiled__.containing_dir` 引入的回归：
Nuitka 对 **onedir** 构建会额外 `STRIP_DIRNAME` 一层（`CodeTemplatesConstants.py` 中
`!ONEFILE_MODE` 分支），使 `__compiled__.containing_dir` 指向 exe 的**父目录**；只有 onefile
构建它才等于原始 exe 目录。而 `build_nuitka.ps1 -Onedir` 的部署是 `.dist` 平铺（带
certifi/cv2/numpy 等包目录），正是 onedir 形态，于是 base 被解析到发布根，SigLIP/OCR 全部加载失败。
旧 v3.2.0 之所以正常，是其旧 `get_base_path` 走 `dirname(__file__)`（Nuitka 不设 `sys.frozen`，
`__file__` 默认 runtime 模式 = exe 目录）。

**做法**：`get_base_path()` 改为按形态分流——仅 `__compiled__.onefile` 为真时才用
`__compiled__.containing_dir`；`sys.frozen`（PyInstaller）用 `dirname(sys.executable)`；
onedir 与源码运行回退 `dirname(__file__)`。四种形态（onefile / onedir / PyInstaller / 源码）
各走正确路径，并在函数注释里写明 Nuitka 该陷阱。

**验证**：`python -m py_compile config.py` ✅ · 源码态 `get_base_path()` 返回
`src/inference-worker` ✅ · lint 无告警。需用 `.\build_nuitka.ps1 -Onedir` 重编产物并端到端
复测一次（启动日志应显示 `…\ai_worker\models\siglip2-base`，带 `ai_worker` 这一层）。

### PR #6 本地采纳：FFmpeg 兜底解码 + 切片段接线 (Batch 53)

**问题**：Windows 下 OpenCV 打不开部分 MKV/AVI 的编解码器 → 视频索引得到 0 帧 →
前端把「发现但解码失败」误报成 "No new files found"。PR #6 提出了 FFmpeg 兜底方案，
但混入了「移除 100 帧试用限制」这条红线，且基于过时代码。故不整体合并，本地重做真金部分。

**做法**（`guard.rs` / `license_cmd.rs` 全程LLM Coding不碰）：

1. **FFmpeg 兜底解码（P1）**：`media/video_extractor.py` 重构为「帧来源 + 帧选择」分离——
   `_iter_opencv_frames`（主路径）与 `_iter_ffmpeg_frames`（兜底）都汇入同一套场景检测 +
   相邻去重 + 三道护栏，OpenCV 打不开/首帧失败即 fallback。FFmpeg 路径沿用 PR 三处细节
   （stderr 写临时文件防管道死锁、`np.frombuffer(...).copy()` 保生命周期、单进程
   `-an/-sn/-dn`），并补 PR 缺的**读超时 watchdog**（30s 读不满一帧即 kill，防卡死
   worker 单线程 REQ/REP）；固定 256×256（与 OpenCV 原始分辨率的口径差异已在 docstring 注明）。
2. **Nuitka 路径**：`config.py::get_base_path` 加 `__compiled__.containing_dir` 分支
   （onefile 下 `sys.executable` 指向临时解压目录，模型/ffmpeg 在原始 exe 同级）。
3. **LGPL 许可证（P1）**：`build_nuitka.ps1` 自动下载 BtbN `win64-lgpl` 固定版本 + SHA256
   校验 + `-buildconf` 断言（命中 `--enable-gpl/nonfree` 即 throw）+ 复制 ffmpeg.exe 到
   worker 同级；绝不取构建机 PATH。补 `THIRD_PARTY_NOTICES.md`（FFmpeg LGPL v2.1+ 条目）与
   根目录 `COPYING.LGPLv2.1`。
4. **错误语义（P2）**：`index_cmd.rs` 补「`added_count == 0 && !failed_paths.is_empty()` 返回
   `Err(ERR_SCAN_ALL_FAILED)`」，把「发现但全解码失败」从误导性的「无新文件」区分出来。
5. **动态超时 + linger（P3）**：`index_cmd.rs` 新增 `index_rpc_timeout_ms`（按批内视频数 ×
   `RPC_INDEX_BUDGET_FRAMES_PER_VIDEO=600` × 250ms 追加预算，替代 PR 的 15 分钟/视频）；
   `zmq_client.rs` 三个 REQ socket 与 `server.py` REP socket 补 `set_linger(0)`，修正
   `BUILD.md` 声称已设但代码没做的不一致。
6. **切片段接线**：死命令 `cut_video_clip` 接上前端——卡片与全屏详情都加「切片段」按钮，
   可调时长（默认 5s，命中帧前后各半），输出到导出目录（未设置回退源视频同目录）。
   Rust 侧 `ffmpeg_cmd.rs` 新增 `ffmpeg_exe()` 复用 `worker_process::ai_worker_dir()`（与
   Python 侧同一处解析，不再依赖 PATH），`cut_video_clip` 自动 `create_dir_all` 输出目录。
   前端新增 `useClip.ts` 单例 + `ClipDialog.vue`，9 语补齐 `clip.*` 与 `ERR_SCAN_ALL_FAILED` 文案。

**收益**：MKV/AVI 编解码失败不再是静默 0 帧；FFmpeg 兜底 + LGPL 护栏 + 明确错误语义闭环；
死命令「切片段」从 PATH 依赖变成打包自带，卡片/全屏可用。

**验证**：`cargo check --features pro` ✅ · `python -m py_compile` ✅ ·
`node scripts/check_i18n_backend.mjs` ✅（9 语 × 147 backend 键，含新增 ERR_SCAN_ALL_FAILED）。
真实 MKV/AVI 样本的 FFmpeg 兜底与切片段端到端验证，需在打包出 ffmpeg.exe 后进行。

**后续已修复（切片段 / 探测健壮性）**：
- 输出文件保留源容器后缀（同容器 `-c copy`），修复 MKV/WebM 里 VP9/AV1/Opus 等
  非 MP4 友好编码硬塞 `.mp4` 会失败的问题（LGPL 构建无 libx264，无法重编码 mp4）。
- `probe_ffmpeg` 从「能 spawn」改为同时校验退出码，损坏但可启动的 ffmpeg 不再误报可用。
- FFmpeg 中途卡死不再整体判失败：0 帧仍 `raise`，已产出部分帧则保留并打警告。
- `build_nuitka.ps1` 的 buildconf 断言踩了 PowerShell 5.1 的坑：`$ErrorActionPreference="Stop"`
  下 `2>&1` 捕获 ffmpeg 的 stderr（banner/配置）会触发 NativeCommandError 中断打包；现临时
  降级 EAP 再恢复。

### 第十二轮 后端文案改为错误码，打通真正的 i18n (Batch 52)

**问题**：后端把英文整句直接 `Err(...)` / `emit` 甩给前端。前端 `useToast.localizeError`
只能对 8 串硬编码英文做子串匹配来「翻译」，其余全部走兜底分支——于是中文界面里
呈现的是「中文分类前缀 + 英文错误原文」，且后端一改文案前端立刻失配。

**做法**：后端只回**稳定的码**，文案完全交给前端 i18n。

1. **新增集中式码表 `src-tauri/src/error.rs`**：定义 146 个码（`ERR_*` 错误 117 /
   `STATUS_*` 状态 22 / `TAG_*` 徽标 6 / `MSG_*` 提示 1）与构造函数 `ec / ec1 / ec2 / ec3`。
   传输格式 `CODE` 或 `CODE|参数0|参数1...`，参数用 `{0}` `{1}` `{2}` 插值，
   底层透传的原生错误用 `{detail}`（= 所有参数重新拼接，避免原文含 `|` 被切碎）。
2. **后端 ~270 处英文文案全部改码**，覆盖 `commands/`、`storage/`、`services/`、`license/`
   四类出口：
   - 命令返回的 `Err(...)` / `map_err(...)`；
   - `scan-progress`、`engine-status` 事件 payload 的 `status` / `message` / `file_path`；
   - 写库并回传前端的 `TextIngestFailure.error`、`ReindexFailure.error`；
   - `matched_tags` 徽标（改为 `TAG_*`）。
   `println!` / `fs_trace!` / `panic!` / `.expect()` 的文案**不动**（只进终端或崩溃路径，不达前端）。
3. **前端解析层**：核心实现 `i18n/index.ts` 的 `localizeCode()`（需直接访问 i18n 实例），
   `src/utils/backendText.ts` 只做转发。按 `|` 切分取码，查 i18n 的 `backend.*`；
   非码内容（真实路径、普通句子）原样返回，故可安全用于 `file_path` 这类
   「有时是路径、有时是状态文案」的字段。查不到时退回原串，绝不把裸 key 给用户看。
4. **接入消费点**：`useToast.localizeError`（删除旧的英文子串白名单）、
   `useScanner`（扫描状态）、`useEngineStatus`（引擎消息）、`ImageCard` / `VideoCard`、
   `exporters.ts`（Markdown 导出的徽标）、`useLicense`（授权错误）。
5. **`t()` 自动解析 `{err}` 参数**：全项目有 ~50 处形如 `t('xxx.failed', { err })` 的模板。
   若逐个包一层极易漏改，故在 `t()` 内部统一把 `err` 过一遍 `localizeCode`——
   一处改动覆盖全部调用点（已确认无模板字符串写法漏网）。
6. **9 语文案全部补齐**：`en` / `zh-CN` / `zh-TW` / `ja` / `ru` / `ko` / `de` / `fr` / `es`
   各 146 个 `backend.*` 键，与后端 `error.rs` 逐一对齐。

#### 核查中发现并修掉的 3 个问题（子 Agent 全量复查）

7. **P0：locale 文件里 `backend` 键重复**。各语言包末尾**本来就有**一个同名 `backend` 块
   （28 个旧键，全项目 0 引用）。`JSON.parse` 对重复键保留最后一个 ⇒ 新码表被整体覆盖，
   用户会看到 `ERR_DB_QUERY_FAILED|database is locked` 这类裸码。
   已删除各文件末尾那个死块（一次性脚本处理，9 个文件）。
8. **`useLicense.friendlyActivationError` 失效**：仍在用旧英文子串匹配（`expired` / `email` /
   `Signature`…），而后端现在回的是大写码，全部落到兜底分支。已改为按码解析。
9. **新增校验脚本 `scripts/check_i18n_backend.mjs`**：核对 9 个语言包的键集合是否与 `en`
   一致、**是否有重复根级键**、以及 `{0}/{1}/{detail}` 占位符是否一一对应。
   其中重复键检测是必需的——`JSON.parse` 会静默吞掉重复键，只看解析成功与否发现不了。

**收益**：后端与文案彻底解耦——改文案不必动 Rust，加语言不必动后端。

**验证**：`cargo check` ✅ · `cargo check --features pro` ✅ · `npm run type-check` ✅ ·
`node scripts/check_i18n_backend.mjs` ✅（9 语 × 146 键，无缺失 / 无多余 / 无占位符差异 /
无重复键）。

### 第十一轮 代码核查修复与补齐 (Batch 51)

依据《第七轮 · 收官复盘》第四节最后一条遗留项（⑪），消除「两条检索命令分叉」。

1. **退役 `search_images` 与 `execute_smart_folder`**：智能文件夹此前走一条与主搜索不一致的
   老检索路径（`search_images`：无分区过滤、无通道明细 `matches`、无 `limit==0` 约定）。
   现旧文件夹由前端把 7 字段映射成「快速关键词 + 通道选择」，与搜索栏共用同一套条件引擎
   （`search_unified`）；`execute_smart_folder` 命令删除，前端不再调用。
2. **语义不变量保持**：旧文件夹的「视觉 OR 字面」融合语义，`search_unified` 与之同口径
   （`is_hit = visual || ocr || note || filename`），映射等价、无行为变化。
3. **`refresh_smart_folder_count` 改走 `search_unified`**：计数与执行同源，`search_images` 的
   最后一个调用方也切走，命令得以整体删除。
4. 同步更新 `src/README.md` / `src-tauri/src/README.md` 的命令清单（移除 `search_images`、
   `execute_smart_folder`）。

**验证**：`cargo check` ✅ · `npm run type-check` ✅。

### 第十轮 代码核查修复与补齐 (Batch 50)

依据《第七轮 · 收官复盘》第四节遗留项，继续补齐「告知的诚实性」与「两套口径收敛」。

#### 一、入库删前预告改为「真预演」（此前报的是全库总数）

1. 后端新增只读命令 `preview_text_ingest`：展开目标文件后，逐 `source_uri` 精确统计
   「将被替换的旧块数」与「会动到的来源文件数」，不再用 `list_text_entries` 的全库总数冒充。
   前端 `useTextEntries.ingestPaths` 改用预演结果：仅当 `will_replace_chunks > 0` 才停下确认，
   **全新文件不再被误拦**、报的数字也不再夸大。
2. 展开逻辑抽取为 `expand_targets`，与 `ingest_text_files` 共用，保证
   「预告说替换 N 块，实际就动 N 块」口径一致。
3. 确认框文案更新为「将替换 N 块（来自 M 个文件）」，9 语同步。

#### 二、智能文件夹计数与执行统一口径

4. `list_smart_folders` 的字面计数从「整串 query 当连续子串裸 `contains`」改为与执行路径
   相同的 `normalize_query`（切词 AND + 全角折叠）+ `match_text`。此前多词 query（如「白天 黑夜」）
   列表几乎必 0、点进去按 AND 命中一堆，数字失真；现在徽标与点进去一致。

#### 三、快速通道全取消的诚实提示

5. 快速搜索的 5 个通道全部取消时，会回退到默认「文件名 + 笔记 + 文字识别」（既定语义），
   现右下角弹一条固定提示如实告知（暂不可关闭，重新勾选任一通道即消失），
   避免「全取消却仍出结果」的困惑。

**验证**：`cargo check` ✅ · `npm run type-check` ✅ · 9 语 JSON 合法且相对 en 无缺失。

### 第九轮 代码核查修复与补齐 (Batch 49)

依据《第七轮 · 收官复盘》逐条实读 Batch 46/47/48 的落地代码后，修复第八轮引入/遗漏的缺陷。
本轮性质同第八轮：**不是加能力，是补齐「加了但没接上」「改了但没跟上」的部分**。

#### 一、发布阻塞（P0）

1. **以图搜图组合条件恒 0 结果**：`search_unified` 有 `limit == 0 = 返回全量` 的约定，
   但 `search_by_image` 没有——`take(0)` 直接截断为空。而 L3 拆叶子对所有叶子都传 `limit: 0`，
   导致「基准图 + 第二个正向条件 / 排除 / 换排序 / 任一满足」时基准图叶子恒空、AND 下整组 0 结果。
   现 `search_by_image` 补上 `limit == 0` 全量分支，与 `search_unified` 同约定。

#### 二、P1 缺陷修复

2. **重命名智能文件夹会静默清空条件行**：`updateSmartFolder` 漏传 `criteriaJson`，被 `?? null`
   补成 null 后后端 `UPDATE` 把 `criteria_json` 写成 NULL。现显式回传原值，改名不再丢条件行。
3. **以图搜图卡片分数恒 🌟99.9%**：以图搜图返回的是「图-图余弦」，量纲与 SigLIP 文本-图余弦不同，
   但前端只按「是否横幅态」判定，seed 条件行结果落到文本阈值、余弦 0.5+ 全被压成 99.9。
   现给以图搜图结果打 `scoreKind='image'`，卡片 / 低分折叠 / 导出统一按图-图余弦映射。
4. **日期快捷项「近 7 天 / 近 30 天」算成未来区间**：原实现 `from = 今天`、`to = from + 7/30 天`，
   而 `index_time` 不可能大于 now ⇒ 实际只命中今天。现改为「过去 6/29 天 + 今天」。
5. **恢复前自动备份可能轮转掉目标备份**：`do_backup` 的轮转按 mtime 删最旧 `manual_backup_*.db`，
   可能把「用户正要恢复的那个备份」挤掉。现 `rotate_backups` 支持排除指定文件，恢复前备份时保护目标备份。

#### 三、P2 缺陷修复

6. **重复的 `</script>`**：`TextEntryManagerDialog.vue` 有两个连续的 `</script>`，编辑残留，已删。
7. **日期「从」输入约束反向**：`ConditionRow.vue` 的「从」日期 `:min="toDateStr"` 应为 `:max`，
   先设「到」之后现在可选更早的「从」。
8. **关掉「上次你在搜 X」提示后重开又出现**：`dismissRestorable` 只清内存 ref、不删
   `localStorage`，现同步落盘清除。

**验证**：`cargo check` ✅ · `npm run type-check` ✅。

**本轮未改（留后续，详见《第七轮 · 收官复盘》第四/五节）**：
- 入库删前预告的真预演接口（当前报的是全库总数，而非「将被替换的块数」）；
- 智能文件夹计数与执行统一口径（列表用裸 `contains`，执行走切词 AND）；
- `search_images` 与 `search_unified` 能力对等（或退役旧路径，消除分叉）；
- 快速通道全取消、扫描报告对话框显示全部批次——经复核属既定语义（空 = 全通道 / 报告本应列全类型），非缺陷，未动。

### 第八轮 代码核查修复与补齐 (Batch 48)

依据《第七轮 · 代码核查与上传前复盘》逐条实读 Batch 46/47 的落地代码后修复。
本轮的性质与前几轮不同：**不是加能力，是把「加了但没接上」「改了但没跟上」的部分补齐**，
并修掉一处功能级回归。

#### 一、修回一处功能级回归（发布阻塞）

1. **以图搜图恢复**：第七轮重写 `performSearch` 时删掉了 `if (isImageSearch) → search_by_image`
   分支且未补回，导致点「以图搜图」与拖拽即搜**必然 0 结果 + 「请至少加一个正向条件」**。
   现把横幅里的目标图转成一个隐式「基准图」条件（`seed`），与其它条件走同一条求值链
   （L1 过滤 + 分页 + 排序 + 部分失败提示全部生效）。用户自己加的基准图条件行优先于横幅。

#### 二、P0 缺陷修复

2. **`search_by_image` 的目录条件恒 0 结果**：`search_cmd.rs` 自带一份私有 `path_key`
   （`\`→`/`），却在拼目录前缀时补了一个 `\`（`storage::path_key` 规范化为 `\`），
   产出 `c:/users/a\` 这种永不匹配的前缀。现统一走 `crate::storage::path_key`，
   删除重复实现，与 `search_unified` 同口径；`search_unified` 另补空串防护
   （`root_path = ""` 曾被 push 成 `"\` 把全部结果滤掉）。
3. **后端移除 `-` 排除词语法**：前端第七轮已删掉布尔解析，但 `normalize_query` / `match_text`
   仍在后端把 `白天 -黑夜` 悄悄按「排除」执行——「用户输入即意图」只落实了一半。
   现彻底移除负向语义：`-` 是普通字符，`e-mail` / `-5°C` / `会议@2024` 都按字面搜。
4. **混合检索的假满分**：`search_unified` 返回的 `score` 是双义的（视觉命中=原始余弦，
   纯字面命中=命中通道数 1/2/3）。「画面」条件原本直接把它当余弦映射，
   导致勾了「画面语义 + 文字识别」时**只靠字面命中的条目拿到 99.9~100 分**并压过真正的视觉命中。
   现 `visual` 条件改走 `getSearchNumericScore`（先看视觉余弦、再看字面命中）。
5. **暂停计时跨扫描污染**：`pausedAt` / `accumulatedPauseMs` / `isPaused` 原先不在
   `startScan` / `stopScan` 复位，暂停过一次之后**第二次扫描的已用时间会减去上一次的暂停时长**
   （可能算出 0 或负数，ETA 直接失真）。现三处复位齐备。

#### 三、搜索能力补齐

6. **智能文件夹保存完整条件行（`criteria_json`）**：新建迁移 `M011_smart_folder_criteria`
   （schema 10 → 11）为 `smart_folders` 加一列。此前保存只取「第一个文字条件 + 有没有画面条件」，
   **排除 / 分条件阈值 / 时间 / 目录 / 批次 / 类型条件全部静默丢弃**。
   现完整存条件行数组；应用文件夹时由**前端条件引擎求值**（与搜索栏同一套求值器），
   因此文件夹与搜索栏的语义彻底一致；旧文件夹（`criteria_json` 为 NULL）仍走原路径，不破坏既有数据。
   列表里的匹配数对条件行文件夹显示「?」（`needs_frontend_eval`）而不是假数字。
7. **`file_type` 条件从死代码变成真功能**：此前 `search_unified` / `search_by_image` 都没有
   `file_types` 参数，类型定义与 `addCondition` 都写了但 UI 不可达。现后端两处接入
   （`file_kind` / `matches_file_types`），条件菜单新增「文件类型（图片 / 视频）」。
8. **`seed`（基准图）条件上 UI**：新增图片选择按钮 + 阈值；这也是以图搜图接入条件行的入口。
9. **排序选择器**：此前只有一行静态文字（用户改不了）。现可选
   「按主条件匹配度 / 按入库时间 / 按文件名」；选后两项时走全量 + 前端排序，保证**跨页全局有序**。
10. **「试算」按钮**：只报命中数、不动结果区——语义条件要跑模型编码，不自动算，由用户点了才算。
11. **日期快捷项 + min/max 联动**：时间条件新增 今天 / 近 7 天 / 近 30 天 / 本月 / 本年；
    「到」早于「从」时自动对齐，不再出现无解释的空结果。
12. **搜索状态随会话恢复**：真正搜过一次才落盘；重开 App 顶部提示「上次你在搜 X」
    并提供「恢复 / 关闭」——**只恢复界面，不自动搜索**（行动由用户触发）。
13. **每页 100 条**：设置页档位 `[4, 8, 16, 32, 50]` → `[4, 8, 16, 32, 50, 100]`。

#### 四、告知与诚实性

14. **文本入库删前预告**：ingest 会按来源替换旧块（破坏性操作），此前只有事后告知。
    现库中已有条目时先停下，弹确认框告知「会替换 N 个条目」并给一次反悔机会。
15. **智能文件夹导入去重**：原先**完全不查重**，重复导入同一份 JSON 会堆积一堆同名文件夹。
    现按名字去重（撞名加序号后缀，绝不覆盖已有定义），并如实回报「导入 N 条，其中 M 条改了名」。
16. **卡片分数不再编百分比**：纯字面命中（只有文件名/笔记/文字识别命中，没有任何视觉余弦）
    是二值事实，后端只回命中布尔。现只标「✓ 命中」，不再显示「85.0% Match」这种无依据的数字；
    65/85/95 仅保留在内部用于排序与阈值。徽章分数线 60 提取为常量，与折叠阈值的关系写进注释。
17. **清理账补上时间戳**：`StartupCleanupReport` 新增 `cleaned_at`（UTC ISO），
    设置页显示「上次启动清理于 YYYY-MM-DD HH:mm」——不可逆的删除连时间都查不到，就不是账，是传言。
18. **扫描报告重试后刷新**：重试索引成功后重新拉批次列表，否则旧失败项仍显示，用户以为「重试没用」。
19. **死代码清理**：`ScoreBar.vue` 的堆叠条样式（模板已不用）、`SearchMode` 的废弃注释清理、
    `list_scan_reports` 的默认行为加注释说明「两个调用方都必须显式表达意图」。

#### 五、i18n

20. 新增 32 个 key（条件类型 / 日期快捷项 / 试算 / 会话恢复 / 入库确认 / 导入改名等），
    **9 语全部补齐**（en / zh-CN / zh-TW / ja / ko / ru / de / fr / es），校验无缺失 key。
21. `src/README.md` 的命令清单与实际签名对齐（`search_by_image` / `search_unified` 的过滤参数、
    `save_smart_folder` 的 `criteriaJson`、`import_smart_folders` 的返回结构）。

**验证**：`cargo check --all-targets` ✅ · `npm run type-check` ✅ · 9 个 locale JSON 校验通过且相对 en 无缺失。

**本轮明确未做（需改协议或需真机验证）**：
- 视频抽帧参数可见可调（需改 `framescout.proto` 并重生成 Rust/Python 两侧绑定）；
- 后端「轻量子查询」（只回 path+score，当前 L3 叶子缓存已缓解翻页重跑）；
- BGE 文本语义接分区过滤（仍实验性，只做互斥）；
- 扫描/入库时的「是否包含子文件夹」开关（当前无此概念，需改协议 + worker 侧）。

### 第七轮 M1'''' 已做但不好修复 (Batch 47)

1. **恢复前自动备份 + 帧数对比**（P0-2）：`restore_backup` 前自动备份当前库（回滚点），
   新增 `preview_restore` 预览；确认框显示「当前 N 帧 → 备份 M 帧，将丢失/增加 X 帧」。
2. **`deleted_old_chunks` 落地**（P0-6）：重新导入文本时告知「替换了 N 个旧块」。
3. **ScoreBar 去假百分比**（P0-7）：字面命中不再画「堆叠占比条」（那需要虚构权重 65），
   改为视觉显示真实匹配度、字面只标「✓ 命中」。
4. **`batch_id` 人类化**：来源批次下拉显示「日期 + 文件数」而非 `scan_20261002_103747`，
   且只列扫描批次（`list_scan_reports` 新增 `batch_type` 过滤）。
5. **`root_path` 修复 + 给 UI**：修 `starts_with` 目录边界（`C:\Users\a` 不再误配 `C:\Users\abc`）
   与 `\`/`/` 规范化；以图搜图接入分区过滤（`search_by_image` 补 `batch_id`/`root_path`/时间参数）。
6. **清理报告可回看**：设置页「自动清理」组显示「上次启动清理了什么」，不再只有一次性 Toast。
7. **暂停 ETA 修正**：暂停期间冻结计时，恢复后把暂停时长从「已用时间」剔除。
8. **以图搜图提示**：横幅明确「搜索关键词不参与以图搜图」。
9. **智能文件夹导出/导入**：导出去 `id`/`match_count` 噪音、走统一导出通道；
   新增 `import_smart_folders` 命令 + 导入按钮，补上「入」。
10. **扫描报告失败列表可复制 + 逐文件重试**：失败文件一键重新索引。

**验证**：`cargo check` ✅ · `npm run type-check` ✅。

**本轮明确延期（详见 `mydocs/FrameScout_第七轮_执行覆盖与延期清单.md`）**：
- 后端「轻量子查询」（只回 path+score）——叶子缓存已缓解翻页重跑，此项 P1 留后续。
- 「试算」按钮、排序切换（按入库时间/文件名）、日期快捷项、搜索状态随会话恢复。
- 视频抽帧参数可见可调（改 proto）——本轮只补「已抽 N 帧」可见日志。
- BGE 文本语义仍是实验性，本轮只做互斥，不接分区过滤。

### 第七轮 M0'''' 逻辑搜索重构：多条件行取代语法方言 (Batch 46)

依据《第七轮·逻辑搜索重构设计》，把「一门要背诵的符号方言」（`-`/`@`/`|`）换成
「一张可点、可删、看得懂的条件表」：

1. **移除语法自动触发**：`parseBooleanQuery` / `runBooleanSearch` 删除，搜索框只做纯关键词，
   搜 `会议@2024` / `e-mail` 不再被静默劫持或 0 结果。
2. **多条件行 UI**（新增 `ConditionRow.vue`）：每一行一个条件（类型 / 值 / 作用域 / 阈值 / ✕），
   组级「全部满足 / 任一满足」切换（不嵌套）；「排除」独立成行，消除 `-黑夜@60` 语义歧义。
3. **取消「搜索模式」「目标通道」全局控件**，全部并入条件行（text/visual/exclude/time/path/batch，
   BGE 文本语义与纯文本条目作为互斥条件）。
4. **求值分层**：时间/目录/批次 = L1 后端过滤合并进单次调用；单一主条件 = L2 单次检索；
   仅「任一满足 / 排除 / 分条件阈值」才 L3 拆叶子。
5. **主条件排序**：分数是「某个具体条件对这张图的匹配度」，不编综合分；卡片标注条件序号。
6. **叶子缓存 + 翻页不重跑**：组合结果缓存，翻页只 slice；部分失败可见（哪个条件失败）。
7. **主搜索框保留「通道选择」**：快速输入框下方保留「🖼️ 画面语义 / 📁 文件名 / 📝 笔记 / 🔠 OCR /
   📝 文本语义」五个勾选项，关键词可按所选通道检索（勾「画面语义」即视觉语义 + 可选字面混合；
   勾「文本语义」走 BGE 文本语义，且与其它四个互斥）；并修复了单一条件检索的翻页 page 硬编码为 1 的问题。
8. **术语「OCR」→「文字识别」**
      - **简体中文（zh-CN）**：所有用户可见的「OCR」术语改为「文字识别」——搜索目标通道、扫描栏语言、卡片按钮「运行文字识别」、得分来源「文字识别」、设置页「文字识别语言」、智能文件夹通道、匹配标签「🔍 文字识别」、导出表头/分节「文字识别文本」等 17 处。
      - **繁体中文（zh-TW）**：同样改为「文字辨識」。
      - **英文（en）**：保留「OCR」——因为英文里 OCR（Optical Character Recognition）是标准通用术语，改成「text recognition」反而不自然，故不改。
      - 保留未动的仅两处技术性名称：`RapidOCR`（模型名）与各 i18n 的 key 名（`ocr`/`runOcr` 等，改了会破坏引用）。

   **补齐之前拉下的 i18n 翻译**
      - 通过脚本比对发现 `ja/ru/ko/de/fr/es` 六语各缺 **92 个 key**（第六轮 + 第七轮新增的「自动清理、恢复、灵敏度、折叠、条件行 cond 整段、报告重试、来源筛选」等）。
      - 已为六语各补齐这 92 条翻译（日语/俄语/韩语/德语/法语/西班牙语），临时脚本用完已删除。

   #### 验证
   - 9 个 locale JSON 全部合法 ✅
   - 六语相对 en 无缺失 key ✅
   - `npm run type-check` ✅

**验证**：`cargo check` ✅ · `npm run type-check` ✅。

### 第六轮 M2''' 备份恢复 / 报告过滤 / 扫描体验 (Batch 45)

1. **备份 restore（补上「出」，结束「单程票」）**：`restore_backup` 校验备份合法性后写
   `pending_restore.txt` 标记，提示「下次启动生效」；启动时 `db.rs::apply_pending_restore`
   在打开数据库前用备份替换主库（此时连接未建立、无锁冲突）。设置页备份项新增「🔄 恢复」+ 二次确认。
2. **各报告搜索/过滤**：幽灵/差异报告本已有过滤，本轮为扫描报告（`ScanReportDialog`）补上
   按批次 id / 类型过滤。
3. **首屏自动加载浏览列表**：引擎就绪后若无搜索词，自动拉第一页，不再「打开是空的」。
4. **扫描暂停 / 继续**：`pause_scan` / `resume_scan` 命令 + `pause_flag`，扫描循环原地等待；
   扫描栏新增 ⏸ 暂停 / ▶ 继续。
5. **智能文件夹导出**：pill 栏新增 ⬇️ 导出（JSON），补上「出」的对称性。

**验证**：`cargo check` ✅ · `npm run type-check` ✅。

**本轮明确延期（详见 `mydocs/FrameScout_第六轮_执行覆盖与延期清单.md`）**：
- 视频抽帧参数可见可调 + 截断告知（需改动 proto 并同步 Rust/Python 两侧绑定，风险较高，留作下一批）。
- 扫描「逐文件重试」按钮（失败文件已可在扫描报告复制后经「Index Files」重试，专用按钮留后续）。
- 报告「排序」控件（搜索/过滤已补，排序留后续）。

### 第六轮 M2''' 分区搜索 + 搜索布尔逻辑与分条件阈值 (Batch 44)

1. **分区搜索（P1-4，全轮次最大未兑现项）**：`search_unified` 新增可选过滤参数
   `batch_id`（来源批次）/ `root_path`（根目录前缀）/ `index_time_from/to`（入库时间范围），
   后端只做事实性过滤、不改打分口径；搜索栏新增「来源批次」下拉 + 入库起止日期。
2. **搜索布尔逻辑与分条件分数阈值**（新需求，守住「后端不做决定」）：前端把表达式
   `白天 -黑夜` / `白天|黑夜` / `白天@75` 解析成叶子子查询，每个叶子仍调 `search_unified`
   （`limit=0` = 返回全部原始命中、不截断），前端做并/交/差的集合组合 + 分条件阈值过滤。
   组合与阈值判定全在前端内存完成，后端不回越界做决定。

**验证**：`cargo check` ✅ · `npm run type-check` ✅。

### 第六轮 M0''' 搜索可解释性与剩余哲学违反项 (Batch 43)

1. **修 ScoreBar 假解释**（`ScoreBar.vue`）：字面命中（OCR/笔记/文件名）是二值命中，后端只回命中布尔、
   没有可比通道分，故图例从伪造的「65%」改为「✓ 命中」，并加诚实说明「不是精确相关度分数」。
2. **相关度灵敏度可设**（`score.ts` + 设置页「🎚️ 相关度灵敏度」宽松/标准/严格）：
   文本语义分段阈值与低分折叠阈值随灵敏度分三档，用户能改变「什么算相关」。
3. **低分自动折叠可关**（设置页「自动折叠低分结果」开关，「不折叠」是一等选项）。
4. **纯负向查询返回可解释空态**：`-cat` 这类查询不再静默 0 结果，前端提示「请至少加一个关键词」。
5. **披露查询改写规则**：搜索框下新增提示行，写明切词 AND、`-` 排除、`|` OR、`词@分数` 阈值、全角折叠。
6. **修智能文件夹 ↻ 3 秒骗局**（`SmartFolderBar.vue`）：刷新态改为由父组件真实完成时移除，
   超 3 秒只把提示切为「仍在进行…」，绝不假装完成。
7. **切模式不再静默重写目标**：`userTouchedTargets` 守卫改为持久化（`targetsTouched`），重开 App 仍记住。
8. **enableOcr 持久化**（此前每次重启静默回到 false）。
9. **以图搜图不销毁用户输入**：`searchByImageAction` / `clearImageSearch` 不再改写/清空搜索框。

**验证**：`npm run type-check` ✅。

### 第六轮 M0''' 哲学合规止血：自动清理给开关并告知 (Batch 42)

依据《FrameScout 第六轮·哲学合规审计》的结论——「系统替你删了东西、定了阈值、改了勾选」，
把已经替用户做过的决定还回给用户。核心是给 9 处自动清理全部补上开关，并让每次清理可被知晓：

1. **新增 `app_settings` 键值表（`M010_app_settings` 迁移，`SCHEMA_VERSION` 9→10）**：
   生命周期清理策略落在 SQLite 而非 localStorage——裁剪发生在启动期、前端尚未就绪，
   localStorage 存在「鸡生蛋」问题。配套 `get_cleanup_settings` / `set_cleanup_settings` 命令。
2. **自动清理三档策略可设**（设置页新增「🧹 自动清理」组）：
   - 事件历史保留天数（默认 90，`-1` = 永不清理）——`db.rs::prune_old_events_and_batches`
   - 备份保留份数（默认 3，`-1` = 不限）——`backup_cmd.rs::rotate_backups`
   - 备份保留天数（默认 30，`-1` = 不限）——`journal.rs::prune_reindex_backups`
3. **自动清理要告知**：`init_db_and_load_memory` 返回 `StartupCleanupReport`
   （事件/批次/冷备份/残行各清了多少），经 `get_startup_cleanup_report` 回传，前端引擎就绪后
   Toast 汇总告知；手动备份轮转删除旧备份时也回传被删文件名并 Toast。
4. **搜索历史条数可设**（此前静默截到 20）：`usePreferences.searchHistoryLimit`，`-1` = 不限。
5. **ingest 删旧块告知**：`TextIngestResult` 新增 `deleted_old_chunks`，重新导入时告知替换了多少旧块。

**验证**：`cargo check` ✅ · `npm run type-check` ✅。

### 索引分段重启 Worker（DirectML 资源保护）(Batch 41)

针对 `onnxruntime-directml 1.24.4` 在 Intel Arc 上连续大量 OCR 推理后资源耗尽崩溃
（`UnicodeDecodeError`，实测约 530 张触发，详见 `mydocs/benchmarks/README.md`）的问题，
在索引流程中引入「分段重启」保底机制：

1. **每 160 张 OCR 重启一次 worker**（`constants.rs::OCR_RESTART_THRESHOLD = 160`）：
   `index_cmd.rs` 的批循环里维护累计计数，OCR 模式下达到阈值即重启 worker 进程，
   彻底重置 DirectML 后端、释放 GPU 资源。纯 SigLIP（不开 OCR）不触发（1000 张不崩）。
2. **worker 句柄入 `AppState`**（`state.rs::worker_process`）：启动时由 `lib.rs` spawn
   后放入共享状态，退出时从中取出 kill，命令层据此可随时重启。
3. **新增 `restart_ai_worker` + `wait_until_ready`**（`worker_process.rs` / `engine_monitor.rs`）：
   `restart_ai_worker` 先 `taskkill /T` 杀掉旧进程树再拉起新进程；`wait_until_ready`
   用同步 PING 轮询等待新 worker 加载模型就绪（最多 90 秒）。
4. **前端「重置引擎」提示**：重启前 `emit("scan-progress", ...)` 一个
   `🔄 Resetting AI engine...` 状态，`ExtractionBus.vue` 直接显示，用户能感知到
   索引过程中的短暂重置（重启通常在数十秒内完成，模型有缓存时更快）。

**验证**：`cargo check` ✅。

### OCR 扫描栏下拉多选 + i18n 收尾 (Batch 40)

1. **扫描栏 OCR 语言改为下拉多选**：`TopActionBar.vue` 的 OCR 语言由 free-text 输入框
   （需手敲 `en, ch_sim` 语言码、易错）改为下拉多选，直接复用 `useOcrLanguages` 的
   语言目录与已选态（底层是 `usePreferences` 单例），与设置页多选列表共享同一份选择、
   `en_opt` 互斥逻辑统一生效；`App.vue` 移除 `ocr-languages` 的 v-model 透传（改为
   composable 单例直取，数据流更干净）。
2. **en_opt 显示名 i18n 化**：新增 `ocr.langEnOpt`（9 语），OCR 语言列表与扫描栏下拉
   里的 en_opt 随界面语言显示为「英文（优化）/ English (optimized) / …」，其余语言
   仍用母语名（native）。
3. **tagline 翻译修正**：9 个非英文 locale 的 `settings.tagline` 前半段
   「克 · 通 · 隱」（`Restraint · Insight · Privacy` 被误译成三个单字）全部改为
   正确翻译——ru / ko / de / fr / es 改为对应语言（Сдержанность · Проницательность ·
   Приватность 等）；zh-CN / zh-TW / ja 改为「克制 · 洞察 · 隐私」「克制 · 洞察 ·
   隱私」「克己 · 洞察 · プライバシー」。

**验证**：`npm run type-check` ✅。

### OCR 纯英文优化模型 (Batch 39)

针对「库里是纯英文、不需要中文」的用户，新增一个**纯英文优化识别模型**的可下载选项：

1. **`ocr_manifest.json` 新增 `en_opt` 语言项**（`en_PP-OCRv4_rec_mobile.onnx` +
   `en_dict.txt`，`tier: downloadable`）：与内置的 `en`（中英合并模型）区分开——纯英文
   模型的字符集只有约 95 类（字母/数字/标点），相比中英合并模型的 ~6600 类，对形近字
   （`l`/`1`、`O`/`0`）误识率更低，代价是**认不了中文**。文件名与 SHA256 取自官方
   RapidAI/RapidOCR `default_models.yaml`（ModelScope v3.10.0），并反向核对了 manifest
   里既有 ch_tra/ja/ko/cyrillic/latin 的哈希均无误。
2. **识别策略互斥**（`useOcrLanguages.ts::toggle`）：`en_opt`（纯英文模型）与
   `en`/`ch_sim`（中英合并模型）互斥——勾选一方自动取消另一方，避免英文文本被两种
   模型重复识别。
3. **文档**：`OCR_MULTILANG.md` 补「纯英文选项 + 场景选型」章节，明确「每个非英文
   多语言模型的字典都已内建拉丁字母+数字，故 X 语言+混排英文由 X 模型单模型覆盖；
   唯「纯英文」场景才需要专门的 `en_opt`」。

**验证**：`npm run type-check` ✅ · manifest 16 语言解析通过 · worker 侧
`available_languages()` 对未下载的 `en_opt` 正确排除、`_group_key('en_opt')='en_opt'`
独立分组。

### OCR 多语言健壮性收尾 (Batch 38)

真机核对 `rapidocr_onnxruntime 1.4.4` 的多语言加载链路时，确认 `rec_model_path` /
`rec_keys_path` 两个 kwargs 为官方支持（`parse_parameters.py` 有映射）；同时修掉两个边界问题：

1. **可用性判定补查字典文件**（`engines/ocr_engine.py::_model_files_present`）：此前只
   检查 rec 模型 `.onnx` 是否存在，未检查字典 `.txt`。而官方 v3.10.0 的 `*_rec_mobile`
   模型**不内嵌字符表**，字典缺失时 `CTCLabelDecode` 会抛 `character must not be None`
   ——语言被判「可用」却静默识别失败。现改为 rec 模型与字典**成对校验**，缺失任一
   即判不可用，前端会提示重新下载而非静默漏识别。
2. **下载命令幂等化**（`commands/model_cmd.rs::download_ocr_model`）：此前 rec 模型
   下载成功、字典下载失败后重试，会从头重下 rec（浪费）。现下载前先判文件是否已
   存在，已存在则跳过，重试只补缺失文件，配合断点续传体验更顺。

另将多语言实现细节与新增语言指引补入 `OCR_MULTILANG.md`（全面重写），并同步更新
worker `README.md`、`BUILD.md` 的目录结构与打包说明（`ocr_manifest.json` 与
`models\ocr\` 的分发）。

**验证**：`cargo check` ✅ · worker 侧 `available_languages()` 实测
`['ch_sim','ch_tra','en','ja']`（en/ch_sim 恒可用、ch_tra/ja 已下载可用、未下载的 ru 正确排除）。

### 国际化扩展与 OCR 多语言识别 — 阶段二 (Batch 37)

兑现「可下载 OCR」的完整机制：用户可在设置页按需下载俄 / 韩 / 德 / 法 / 西五种识别
模型，含下载进度可视化、失败重试与断点续传、磁盘空间检查、SHA-256 完整性校验、删除，
模型统一落盘到既有 `models/ocr/` 目录。

1. **Rust 下载命令（`model_cmd.rs` 扩展 + 新依赖）**：新增 `download_ocr_model`；
   引入 `reqwest`（rustls-tls + stream）、`sha2`、`sysinfo` 三个依赖。下载走「官方
   镜像优先、自建镜像兜底」（manifest 的 `urls` 有序列表），HTTP Range 断点续传
   （写 `.part`，失败保留以便续传），下载后 SHA256 校验通过再原子重命名；下载前按
   目标盘可用空间做检查（`sysinfo`）。
2. **下载进度事件**：Rust 侧按约 200ms 节流发射 `ocr-download-progress`
   （语言码 / 已下载 / 总量 / 百分比），前端监听并渲染进度条。
3. **前端下载交互（`OcrLanguageList.vue` + `useOcrLanguages.ts`）**：可下载语言
   未下载时显示「下载模型」按钮，下载中显示进度条 + 百分比，失败后按钮变「重试」
   （断点续传，不从头开始）；内置随包语言（ch_tra/ja）与已下载模型显示删除按钮
   （二次确认）。下载完成自动刷新列表并 Toast 提示。

**验证**：`npm run type-check` ✅ · `cargo check` ✅（无告警）（但是去除了 SHA256 独立重复校验，下载时已做）。

### 国际化扩展与 OCR 多语言识别 — 阶段一 (Batch 36)

把界面语言从「英 / 简中」扩到 9 种，并把 OCR 识别语言从「英 / 简中」扩展为
「4 内置 + 5 可下载 + 6 规划占位」三档，设置页由下拉改为多选列表。本批为阶段一：
i18n + 语言列表 UI + 内置 OCR（可下载 OCR 的下载/校验/续传机制在下一批）。

1. **界面语言 9 种（i18n）**：新增 `zh-TW` / `ja` / `ru` / `ko` / `de` / `fr` / `es`
   共 7 个语言包（`src/FrameScout-UI/src/i18n/locales/`，每包 522 键、与 en 严格同构），
   `i18n/index.ts` 的 `SUPPORTED_LOCALES` 与 `setLocale` 由二元判断改为 9 语映射，
   设置页「语言」下拉扩至 9 项（以母语名显示）。英文仍为默认语言。
2. **OCR 语言目录单一事实源（`ocr_manifest.json`）**：新增
   `src/inference-worker/ocr_manifest.json`，集中描述语言码 / 分层（builtin_wheel /
   builtin_bundle / downloadable / planned）/ rec 模型与字典文件名 / 官方下载 URL /
   SHA256。识别模型与哈希逐项取自 RapidAI/RapidOCR 官方模型库（ModelScope v3.10.0 的
   `default_models.yaml`），确保「真官方、无差错」。
3. **worker 多语言引擎改造**（`engines/ocr_engine.py` + `config.py`）：det/cls 通用、
   rec 随语言分组——en/ch_sim 共用 wheel 默认权重，其余语言从 `models/ocr/<code>/`
   加载对应 rec 模型与字典，逐组跑 OCR 后合并行级结果；`SUPPORTED_LANGUAGES` 由硬编码
   集合改为按「磁盘上 rec 模型是否真实存在」动态判定，下载/删除后无需重启即生效。
4. **内置繁中 / 日文识别模型（可删）**：ch_tra、ja 作为 `builtin_bundle` 随安装包
   内置，新增 `scripts/download_ocr_models.py` 从官方源下载并 SHA256 校验；
   `build_nuitka.ps1` 复制 `ocr_manifest.json` 与 `models\ocr\` 到产物旁。
5. **Rust 新增模型命令**（`commands/model_cmd.rs` + `services/paths.rs`）：模型路径
   解析（与 worker `get_base_path()` 同源，避免两份事实漂移）、`list_ocr_languages`
   返回三档语言与可用性、`delete_ocr_model` 带安全护栏（仅删 `deletable` 语言、
   只动 `models/ocr/<code>/`）。
6. **OCR 语言选择器改为多选列表**：设置页下拉 → `OcrLanguageList.vue` 多选列表，
   三档分组 + 状态徽标（内置识别 / 识别模型可下载（云朵图标）/ 即将推出），支持多语言
   同时开启；内置的 ch_tra/ja 与已下载模型可删（二次确认）。偏好数据由单值字符串
   数组化（`usePreferences.ocrLanguages`），并迁移旧 localStorage 逗号串。

**验证**：`npm run type-check` ✅ · `cargo check` ✅（1 处 dead_code 告警为下一批
下载命令待用字段）。

### worker 源码启动 fallback (Batch 35 · 采纳 PR #5)

1. **worker 启动 fallback**：找不到 `ai_worker.exe` 时（如 `tauri dev` 未做 Nuitka 打包），
   回退到从 `.venv` 源码直接运行 `main.py`，不再 panic；可用 `FRAMESCOUT_WORKER_PYTHON`
   环境变量覆盖解释器。
2. **退出进程树清理**：应用退出时用 `taskkill /F /T` 杀整棵进程树，避免残留子进程占用
   端口（issue #3 的一类根因）。
3. **来源**：采纳社区 PR #5（kiwiqq-jpg）的第二个 commit（cherry-pick `9c48533`）。

### 试用限制透明化 (Batch 34 · issue #4 收尾)

1. **README 补试用限制说明**：下载提示处明确「预编译版含 100 帧免费试用、视频按抽帧
   数计入」，并说明「试用限制同在源码中，可修改/移除后自行构建」，中英同步。回应
   issue #4 及后续邮件：此前文档只写 "Free & open source"、未提前说明试用
   上限，用户撞顶后误以为是隐瞒。
2. **试用上限报错口径统一**：`guard.rs` 撞顶报错由 "{} images" 改为 "{} indexed frames"，
   并把未落地的 "🚀 Upgrade to Pro" 引导改为「modify or remove the trial limit and build
   your own version」，与 README 口径一致（此前限制的是帧数 `memory_db.len()`，文案却写
   images，视频抽帧会提前触顶）。
3. **构建配置补全**：`tauri.conf.json` 增加 `build.windows.staticVCRuntime: true`，
   替代已废弃的 `STATIC_VCRUNTIME` 环境变量写法，消除构建警告。
4. **版权主体去 "Inc."**：`LICENSE` / `lib.rs` / `App.vue` 的版权由 `AetherFlow Labs Inc.`
   改为 `AetherFlow Labs`，与 README 一致（公司尚未注册为 Inc.）。

### 端口冲突修复 (Batch 33 · 5555 → 16666)

1. **服务端口 5555 → 16666**：用户（issue #3）报告引擎无法启动，日志显示
   `zmq.error.ZMQError: Permission denied (addr='tcp://127.0.0.1:5555')`——其 Windows
   的 Hyper-V/WSL 把 5555 保留进了 excluded port range，导致 worker 绑定失败。已把
   worker 与 Rust 侧的端口统一改为 `16666`（`config.py`、`zmq_client.rs`、
   `engine_monitor.rs`、`benchmark.py` 及文档），避开高频冲突端口。
2. **长远的彻底方案——端口冲突自动 fallback（记入 v3.3 规划）**：换端口只是降低碰撞
   概率，并非绝对治本（Windows 保留端口范围是动态分配的，任何端口理论上都可能被保留）。
   彻底解法是 worker 绑定失败时自动降级到下一个可用端口，并把实际端口回传给 Rust 侧
   （需扩展通信协议）。此项列为 v3.3「开箱即用健壮性」的长期目标，不在本轮完成。

## [3.2.0] - 2026-10-08

### 首次开源发布收尾 (Batch 32 · 发布修复与文档核查)

1. **修复 release 构建黑屏**：`tauri.conf.json` 的 CSP `script-src` 缺 `'unsafe-eval'`，
   而 vue-i18n 运行时编译翻译消息用到 `new Function()`，release 下被 CSP 拦截抛出
   `EvalError`，导致前端整体崩溃、界面纯黑；现于 `script-src` 加入 `'unsafe-eval'`
   （离线桌面应用，eval 仅来自可信的 vue-i18n）。
2. **vite 构建目标降级**：`build.target` 由 `esnext` 改为 `chrome105`（Windows）/
   `safari13`（其他平台），避免产物使用 WebView2 内核不兼容的最新 ES 语法（Tauri 官方推荐）。
3. **资源路径改为相对**：`base: './'`，release 下 `tauri://localhost` 稳定加载 `assets/*`
   （此前绝对路径在 dev 正常、release 存在解析风险）。
4. **bundle identifier 修正**：`com.framescout.app` → `com.framescout.desktop`，
   消除「identifier 以 `.app` 结尾与 macOS bundle 扩展冲突」的构建警告。
5. **根 README 重写（中英双语）**：修正过时事实（RapidOCR / 版本号 / 仓库结构 / 混合评分
   归属前端），按「诚实但克制」扩充「已知限制」、路线图按真实完成度标注，中英严格同步。
6. **品牌统一为 Global Edition**：完整功能版、发布于社区，故将「Community Edition / 社区版」
   统一为「Global Edition / 全球版」。
7. **LICENSE 替换为完整 Apache 2.0 全文**（保留 `Copyright 2026 AetherFlow Labs Inc.`），
   使 GitHub 能正确识别许可证。
8. **文档合并 / 更名 / 清理**：删除过时的 `README_AI.md`、`落地说明.md` 并入前端 README、
   `RAPIDOCR_MULTILANG.md` 更名改写为 `OCR_MULTILANG.md`、更新 `TREE.md` 与各子目录 README。
9. **.gitignore 增补**：排除 Nuitka 崩溃报告（`nuitka-crash-report.xml`）、`.codebuddy/`、
   Pro 签发工具链（`developer_public_key.pem` / `license_gen.py`），仅保留公开仓库应公开内容。

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
