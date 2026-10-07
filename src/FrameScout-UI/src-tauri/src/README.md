# FrameScout — Rust/Tauri 后端模块结构

```
src-tauri/src/
├── main.rs                   # 桌面端薄壳入口：仅调用 framescout_ui_lib::run()
├── lib.rs                    # 库入口：模块聚合、Tauri Builder、生命周期挂钩
├── constants.rs              # 全局常量 (FREE_TRIAL_LIMIT, VECTOR_DIM)
├── proto.rs                  # Protocol Buffers 宏引入与映射
│
├── model_code/                   # 前后端数据传输对象 (DTO) 与 App 状态
│   ├── mod.rs
│   ├── state.rs              # AppState (全局共享状态)
│   ├── search.rs             # SearchResult, PagedResponse
│   ├── smart_folder.rs       # SmartFolder 结构体
│   ├── cluster.rs            # ClusterGroup (视觉聚类结果)
│   └── progress.rs           # ProgressPayload (扫描进度事件载荷)
│
├── license/                  # 授权与限制系统
│   ├── mod.rs
│   ├── verifier.rs           # Ed25519 签名验证与本地 .lic 读取 (Pro 特性)
│   └── guard.rs              # TrialGuard (试用期额度限制守卫)
│
├── storage/                  # 数据持久化与内存计算
│   ├── mod.rs
│   ├── vector_matrix.rs      # FlatVectorMatrix (连续内存 768D 向量点乘检索)
│   ├── db.rs                 # SQLite 初始化、WAL 配置、建表、冷启动载入与事件裁剪
│   ├── migrations.rs         # 版本化迁移链 (M001~M009，含 FTS5 / 索引)
│   ├── journal.rs            # 账簿：观测 / 事件 / 批次 (files / file_events / batches)
│   ├── path_util.rs          # 路径规范化 + path_key 大小写折叠
│   ├── text_store.rs         # 纯文本条目读写 (FTS5 + LIKE 回退)
│   └── ocr_store.rs          # media_ocr_entries 行级 OCR 写入
│
├── services/                 # 外部服务与通信层
│   ├── mod.rs
│   ├── worker_process.rs     # AI 子进程管理 (查杀旧进程、路径查找、静默启动)
│   ├── zmq_client.rs         # request_vector (ZMQ Protobuf 通信封装)
│   └── engine_monitor.rs     # wait_for_engine_and_notify (引擎就绪健康检查)
│
└── commands/                 # Tauri 前端调用命令 (按业务拆分)
    ├── mod.rs                # 汇总并对外导出所有 Tauri Commands
    ├── index_cmd.rs          # scan_folder, index_files, cancel_scan
    ├── search_cmd.rs         # search_images, search_by_image, search_text, ping_engine
    ├── search_unified_cmd.rs # search_unified (统一跨通道融合，已接入前端)
    ├── ocr_cmd.rs            # run_ocr_for_selected_files
    ├── smart_folder_cmd.rs   # 智能文件夹增删改查、重命名、计数刷新与动态执行
    ├── cluster_cmd.rs        # cluster_similar_images (medoid + 随机采样 + 快照)
    ├── file_cmd.rs           # update_note, list_all_files, get_all_files
    ├── ghost_cmd.rs          # preview_ghosts, apply_ghost_action (含 NAS 守卫 + 级联删除)
    ├── diff_cmd.rs           # scan_diff (差异报告)
    ├── reindex_cmd.rs        # reindex_files
    ├── report_cmd.rs         # list_scan_reports, list_timeline, last_year_today_count
    ├── text_cmd.rs           # ingest_text_files (chunk_text 分块)
    ├── backup_cmd.rs         # backup_database, verify_integrity, list_backups
    ├── ffmpeg_cmd.rs         # probe_ffmpeg, cut_video_clip (backend ready; frontend wiring pending)
    └── license_cmd.rs        # activate_pro_license, get_license_status
```
