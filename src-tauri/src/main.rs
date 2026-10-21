// [INPUT]: Windows 平台边界、Tauri 生命周期、发布构建 custom-protocol、旧数据迁移、线路设置/检查/门禁、身份、CLI 安装管理、浏览器、地区、系统与界面语言命令及重启插件。
// [OUTPUT]: CC-Guard 窗口（开机启动时留在托盘）、Claude 线路状态托盘、SQLite 本地身份暂存/恢复、CLI 卸载/重装与随界面语言切换的托盘菜单，以及 CI 的固定启动阶段诊断。
// [POS]: 应用编排层；只注册 src/contract.ts 列出的命令。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
// ----- 发布包必须内嵌前端资源，禁止依赖开发服务 -----
#[cfg(all(not(debug_assertions), not(feature = "custom-protocol")))]
compile_error!("发布构建需要 custom-protocol；使用 tauri build 或 cargo build --release --features custom-protocol。");
mod browser;
mod claude_cli;
mod cleaner;
mod identities;
mod native;
mod progress;
mod regions;
mod system;
mod ui_language;
#[cfg(test)]
mod updater_tests;
mod line;
mod migrate;
mod mihomo;
mod tray;
mod windows;
use tauri::{Emitter, Manager};
fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        // The page polls only while it can be seen; the tray re-checks the line in the background.
        let _ = window.emit("window-visible", true);
    }
}
pub(crate) fn startup_trace(stage: &str) {
    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
        // CI 只记录固定启动阶段，不读取文件正文、参数或凭据。
        let path = std::env::temp_dir().join("ccbaga-ci-startup.log");
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            use std::io::Write;
            let _ = writeln!(file, "{stage}");
        }
    }
    let _ = stage;
}
fn main() {
    startup_trace("host entry");
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if args.iter().any(|arg| arg == "--browser") {
                browser::open_urls(app.clone(), args);
                return;
            }
            // A sign-in start while already running must not pop the window.
            if args.iter().any(|arg| arg == "--tray") { return; }
            show(app)
        }))
        .setup(|app| {
            startup_trace("host setup");
            // Before anything touches the data directory: carry over the pre-rename data once.
            migrate::run(app.handle());
            app.manage(ui_language::UiLanguage::load(app.handle()));
            app.manage(cleaner::Cleaner::default());
            app.manage(browser::Browser::default());
            app.manage(line::Line::default());
            if std::env::args().any(|arg| arg == "--browser") {
                browser::open_urls(app.handle().clone(), std::env::args().collect());
            }
            tray::build(app.handle())?;
            tray::watch(app.handle().clone());
            // Started at sign-in: stay in the tray until the user opens the window.
            if !std::env::args().any(|arg| arg == "--tray") {
                show(app.handle());
            }
            startup_trace("host ready");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                let _ = window.emit("window-visible", false);
            }
        })
        .invoke_handler(tauri::generate_handler![
            line::settings_read,
            line::settings_write,
            line::line_check,
            line::line_connections,
            line::line_nodes,
            line::launch,
            line::workspace_choose,
            ui_language::ui_language_read,
            ui_language::ui_language_write,
            cleaner::cleaner_stop,
            identities::identities_read,
            identities::identities_browser,
            identities::identities_stop,
            identities::identities_switch,
            identities::identities_rename,
            identities::identities_delete,
            claude_cli::claude_cli_read,
            claude_cli::claude_cli_manage,
            system::system_read,
            system::system_set_timezone,
            system::system_disable_location,
            browser::browser_read,
            browser::browser_launch,
        ])
        .build(tauri::generate_context!())
        .expect("无法启动 CC-Guard")
        .run(|app, event| match event {
            tauri::RunEvent::Exit => browser::forget_start_url(app),
            _ => {}
        });
}
