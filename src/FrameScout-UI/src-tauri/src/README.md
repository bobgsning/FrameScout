# FrameScout — Rust/Tauri 后端模块结构

```
src-tauri/src/
├── main.rs                   # 桌面端薄壳入口：仅调用 framescout_ui_lib::run()
├── lib.rs                    # 库入口：模块聚合、Tauri Builder、生命周期挂钩
├── constants.rs              # 全局常量 (FREE_TRIAL_LIMIT, VECTOR_DIM)
├── proto.rs                  # Protocol Buffers 宏引入与映射
│
├── models/                   # 前后端数据传输对象 (DTO) 与 App 状态
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
│   └── db.rs                 # SQLite 初始化、WAL 配置、建表与冷启动载入
│
├── services/                 # 外部服务与通信层
│   ├── mod.rs
│   ├── worker_process.rs     # AI 子进程管理 (查杀旧进程、路径查找、静默启动)
│   ├── zmq_client.rs         # request_vector (ZMQ Protobuf 通信封装)
│   └── engine_monitor.rs     # wait_for_engine_and_notify (引擎就绪健康检查)
│
└── commands/                 # Tauri 前端调用命令 (按业务拆分)
    ├── mod.rs                # 汇总并对外导出所有 Tauri Commands
    ├── index_cmd.rs          # scan_folder, index_files
    ├── search_cmd.rs         # search_images, search_by_image, ping_engine
    ├── ocr_cmd.rs            # run_ocr_for_selected_files
    ├── smart_folder_cmd.rs   # 智能文件夹增删改查与动态执行
    ├── cluster_cmd.rs        # cluster_similar_images
    ├── file_cmd.rs           # update_note, clean_ghosts, list_all_files, get_all_files
    └── license_cmd.rs        # activate_pro_license, get_license_status
```
