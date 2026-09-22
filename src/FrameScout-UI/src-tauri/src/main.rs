#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// 桌面端薄壳入口：全部逻辑位于 framescout_ui_lib 库中
fn main() {
    framescout_ui_lib::run()
}
