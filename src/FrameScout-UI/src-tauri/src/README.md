# FrameScout — Rust/Tauri 后端模块结构

```
src-tauri/src/
├── main.rs                   # 桌面端薄壳入口：仅调用 framescout_ui_lib::run()
├── lib.rs                    # 库入口：模块聚合、Tauri Builder、生命周期挂钩、invoke_handler 注册
├── constants.rs              # 全局常量 (FREE_TRIAL_LIMIT, VECTOR_DIM)
├── proto.rs                  # Protocol Buffers 宏引入与映射
├── error.rs                  # 集中式错误码/状态码注册表（后端→前端 i18n 契约，CODE|param 格式）
├── logging.rs                # fs_trace! trace 级日志宏（扫描/批次/搜索关键字段）
├── time_util.rs              # 轻量 UTC 时间戳（备份文件名、扫描批次号；不引 chrono）
│
├── model_code/                   # 前后端数据传输对象 (DTO) 与 App 状态
│   ├── mod.rs
│   ├── state.rs              # AppState (全局共享状态)
│   ├── search.rs             # SearchResult, PagedResponse
│   ├── smart_folder.rs       # SmartFolder 结构体（含 criteria_json 条件行）
│   ├── cluster.rs            # ClusterGroup (视觉聚类结果)
│   ├── progress.rs           # ProgressPayload (扫描进度事件载荷)
│   ├── frame.rs              # FrameResult / OcrLine (编码结果与行级 OCR)
│   ├── ghost.rs              # GhostItem / GhostActionResult (幽灵清理预览与行动)
│   ├── diff.rs               # 差异报告四分类 (新增/修改/消失/缺向量)
│   ├── report.rs             # BatchReport / TimelineDay (扫描报告与时间轴)
│   └── text.rs               # 纯文本通道的入库结果 / 失败明细
│
├── license/                  # 授权与限制系统
│   ├── mod.rs
│   ├── verifier.rs           # Ed25519 签名验证与本地 .lic 读取 (Pro 特性)
│   └── guard.rs              # TrialGuard (试用期额度限制守卫)
│
├── storage/                  # 数据持久化与内存计算
│   ├── mod.rs
│   ├── vector_matrix.rs      # FlatVectorMatrix (连续内存 768D 向量点乘检索)
│   ├── vector_blob.rs        # 向量 BLOB 编解码（小端序平面 f32 的唯一定盘处）
│   ├── db.rs                 # SQLite 初始化、WAL 配置、建表、冷启动载入、事件裁剪与待恢复迁移
│   ├── migrations.rs         # 版本化迁移链 (M001~M011，含 FTS5 / 性能索引 / app_settings / 条件行)
│   ├── journal.rs            # 账簿：观测 / 事件 / 批次 (files / file_events / batches)
│   ├── path_util.rs          # 路径规范化 + path_key 大小写折叠
│   ├── text_store.rs         # 纯文本条目读写 (FTS5 + LIKE 回退)
│   ├── ocr_store.rs          # media_ocr_entries 行级 OCR 写入
│   └── app_settings.rs       # app_settings 键值表（自动清理策略：事件保留 / 备份轮转）
│
├── services/                 # 外部服务与通信层
│   ├── mod.rs
│   ├── worker_process.rs     # AI 子进程管理 (查杀旧进程、路径查找、静默启动)
│   ├── zmq_client.rs         # request_vector (ZMQ Protobuf 通信封装，REQ socket linger=0)
│   ├── engine_monitor.rs     # wait_for_engine_and_notify (引擎就绪健康检查)
│   └── paths.rs              # worker / models / ocr_manifest 路径解析（与 Python config.py 同一口径）
│
└── commands/                 # Tauri 前端调用命令 (按业务拆分)
    ├── mod.rs                # 汇总并对外导出所有 Tauri Commands
    ├── index_cmd.rs          # scan_folder, index_files, cancel_scan, pause_scan, resume_scan
    ├── search_cmd.rs         # search_by_image, search_text, search_text_entries, insert_text_entry, ping_engine
    ├── search_unified_cmd.rs # search_unified (统一跨通道融合 + 分区过滤，已接入前端)
    ├── ocr_cmd.rs            # run_ocr_for_selected_files
    ├── smart_folder_cmd.rs   # 智能文件夹增删改查、重命名、计数刷新、导入导出与条件行动态执行
    ├── cluster_cmd.rs        # cluster_similar_images (medoid + 随机采样 + 快照)
    ├── file_cmd.rs           # update_note, list_all_files, get_all_files
    ├── ghost_cmd.rs          # preview_ghosts, apply_ghost_action (含 NAS 守卫 + 级联删除)
    ├── diff_cmd.rs           # scan_diff (差异报告)
    ├── reindex_cmd.rs        # reindex_files
    ├── report_cmd.rs         # list_scan_reports, list_timeline, last_year_today_count, list_files_by_day
    ├── text_cmd.rs           # ingest_text_files, preview_text_ingest, list_text_entries, delete_text_entries
    ├── backup_cmd.rs         # backup_database, restore_backup, preview_restore, verify_integrity, list_backups, delete_backup
    ├── ffmpeg_cmd.rs         # probe_ffmpeg, cut_video_clip (兜底解码器定位 + 切片段，已接入前端)
    ├── license_cmd.rs        # activate_pro_license, get_license_status
    ├── export_cmd.rs         # export_file (写入用户配置的导出目录，返回完整路径)
    ├── model_cmd.rs          # list_ocr_languages, download_ocr_model, delete_ocr_model (读 ocr_manifest.json)
    └── settings_cmd.rs       # get_cleanup_settings, set_cleanup_settings, get_startup_cleanup_report
```

## 约定

- **错误码契约**：所有到达前端的错误/状态都是 `error.rs` 中的稳定码（`CODE` 或 `CODE|param0|param1`），
  文案由前端 i18n 按码渲染；`println!` / `fs_trace!` / `panic!` 不进前端，可保留可读整句。
- **字节序约定**：`frame_vectors.vector_f32` 一律小端序平面 f32（768 维 = 3072 字节），
  读写只准经过 `storage/vector_blob.rs`。
- **worker 路径单一事实源**：`services/paths.rs` 与 Python 侧 `config.py::get_base_path()`
  解析同一份 `ocr_manifest.json` 与 `models/`，避免两份硬编码漂移。
