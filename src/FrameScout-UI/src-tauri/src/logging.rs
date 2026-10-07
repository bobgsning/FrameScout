// =========================================================================
//  trace 级日志（v3.2.0 工程纪律）
// =========================================================================
//  目的：扫描完成 / 每批入库完成 / 每次搜索完成各留一条带关键字段的记录。
//  出问题时能直接定位到「哪一批、多少文件、耗时多久」，不必从新代码重新查起。
//
//  实现取舍：项目当前统一用 `println!` 输出（Tauri 开发模式下直接进终端），
//  此处不再引入 log / tracing 依赖，只提供一个同源的 `fs_trace!` 宏。
//  将来若接 env_logger / tracing，把宏体换掉即可，调用点一行不改。

/// trace 开关。置 false 可整体静默（仍保留普通 println 提示）。
pub const TRACE_ENABLED: bool = true;

/// 打一条 trace 级日志。用法与 `println!` 一致，输出统一带 `[trace]` 前缀。
#[macro_export]
macro_rules! fs_trace {
    ($($arg:tt)*) => {
        if $crate::logging::TRACE_ENABLED {
            println!("[trace] {}", format_args!($($arg)*));
        }
    };
}
