// [INPUT]: 随包 Windows 指纹引擎、配置/资料边界、真实浏览器观测、宿主界面语言与共享操作日志。
// [OUTPUT]: 有界绑定身份上下文与浏览器设置/启动/清理和环境核验；宿主重启重新连接现有浏览器，回报时间明确标识通讯状态；上下文提供当前界面语言；launch.args 与 SQLite 当前身份共用独占资料/指纹参数，恢复期间阻断启动。
// [POS]: 独立浏览器模块的唯一编排入口；改变资料前必须退出浏览器；不改系统默认浏览器。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
pub(crate) mod config;
mod launch;
pub(crate) mod network;
mod profile;
mod workspaces;
use launch::launch_locked;
pub(crate) use workspaces::delete_saved;
pub(crate) use workspaces::identity_action;
pub(crate) fn configure_identity(
    app: &tauri::AppHandle,
    reporter: &Reporter,
    action: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    workspaces::transition(app, reporter, true, action)
}
include!("windows.rs");
pub(crate) mod server;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod workspace_tests;
use crate::progress::{Progress, Reporter};
use config::{Config, Machine};
use network::{Check, Exit, Report};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
};
use tauri::{ipc::Channel, Manager};
#[derive(Default)]
pub struct Browser {
    gate: Mutex<()>,
    pub(crate) warning: Mutex<Option<String>>,
    session: Mutex<Option<Session>>,
    host: Mutex<Option<Exit>>,
}
struct Session {
    token: String,
    received_at: Option<u64>,
    report: Option<Report>,
}
#[derive(Clone, Serialize)]
pub struct Context {
    identity: crate::identities::browser_link::BrowserIdentity,
    ui_language: String,
    exit_services: &'static [&'static str],
    region: Option<crate::regions::Target>,
    config: Config,
    machine: Machine,
    engine_version: String,
}
#[derive(Serialize)]
pub struct Snapshot {
    ui_language: String,
    exit_services: &'static [&'static str],
    region: Option<crate::regions::Target>,
    config: Config,
    machine: Machine,
    engine_version: String,
    engine_ready: bool,
    running: bool,
    profile: String,
    identity_linked: bool,
    cache: Vec<profile::CacheItem>,
    cache_error: String,
    report: Option<Report>,
    report_received_at: Option<u64>,
    machine_exit: Option<Exit>,
    checks: Vec<Check>,
}
fn identity_context(
    app: &tauri::AppHandle,
) -> Result<crate::identities::browser_link::BrowserIdentity, String> {
    let root = root(app)?;
    let vault = root
        .parent()
        .ok_or("应用目录不可用。")?
        .join("identity-vault");
    workspaces::database(&root)?.browser_identity(&vault, &crate::cleaner::identity::root()?)
}
fn root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|_| "应用目录不可用。")?
        .join("browser"))
}
/// The current identity's browser config, always on the saved Claude-only port (launches still pass line::gate).
fn managed_config(app: &tauri::AppHandle, settings: &Path) -> Result<Config, String> {
    let mut config = config::load(settings)?;
    config.proxy = crate::line::saved_endpoint(app)?;
    Ok(config)
}
/// 当前身份的浏览器启动参数快照（与实测启动相同，只去掉起始页），身份切换期间用 block/unblock 标记。
fn write_launch_args(
    root: &Path,
    config: &Config,
    machine: &Machine,
    profile: &Path,
    version: &str,
) -> Result<(), String> {
    let mut args = config::arguments(config, machine, profile, "", version);
    args.pop();
    crate::identities::private::directory(root)?;
    crate::identities::private::write(
        &root.join("launch.args"),
        (args.join("\n") + "\n").as_bytes(),
    )?;
    workspaces::unblock(root)
}
fn refresh_launch_args(app: &tauri::AppHandle) -> Result<(), String> {
    let root = root(app)?;
    workspaces::ensure(&root)?;
    let settings = workspaces::scope(&root)?;
    let profile = settings.join("profile");
    profile::check_path(&root, &profile)?;
    let config = managed_config(app, &workspaces::scope(&root)?)?;
    let machine = config::machine()?;
    config.validate(&machine)?;
    let engine = engine(app)?;
    write_launch_args(
        &root,
        &config,
        &machine,
        &profile,
        &packaged_version(app, &engine),
    )?;
    workspaces::unblock(&root)
}
/// 新建起始页会话：随机令牌回环服务 + 写入 browser/start.url（0600），供 CCBrowser 入口在每次全新启动时作为首个标签打开。
fn open_session(
    app: &tauri::AppHandle,
    config: &Config,
    machine: &Machine,
    engine_version: String,
) -> Result<String, String> {
    use std::io::Write;
    let state = app.state::<Browser>();
    let listener = server::bind()?;
    let port = listener
        .local_addr()
        .map_err(|_| "本地端口不可用。")?
        .port();
    let token = server::nonce()?;
    let page = format!("http://127.0.0.1:{port}/{token}/browser.html");
    let context = Context {
        identity: identity_context(app)?,
        ui_language: crate::ui_language::ui_language_read(app.clone()),
        exit_services: &network::SERVICES,
        region: crate::regions::target(&config.language(machine), &config.timezone(machine)),
        config: config.clone(),
        machine: machine.clone(),
        engine_version,
    };
    let web = web(app)?;
    *state.session.lock().unwrap() = Some(Session {
        token: token.clone(),
        report: None,
        received_at: None,
    });
    server::start(app.clone(), listener, token, web, context);
    let root = root(app)?;
    fs::create_dir_all(&root).map_err(|_| "无法创建浏览器目录。")?;
    let temp = root.join("start.url.tmp");
    let mut options = fs::OpenOptions::new();
    options.create(true).write(true).truncate(true);
    let mut file = options.open(&temp).map_err(|_| "无法写入起始页地址。")?;
    file.write_all(format!("{page}\n").as_bytes())
        .map_err(|_| "无法写入起始页地址。")?;
    fs::rename(&temp, root.join("start.url")).map_err(|_| "无法写入起始页地址。")?;
    Ok(page)
}
/// 宿主退出时删除 start.url，避免 CCBrowser 入口读到失效令牌。
pub fn forget_start_url(app: &tauri::AppHandle) {
    if let Ok(root) = root(app) {
        let _ = fs::remove_file(root.join("start.url"));
    }
}
/// 引擎自带 version.txt；引擎未准备好时退回打包资源里的版本号。
fn packaged_version(app: &tauri::AppHandle, engine: &Path) -> String {
    if engine.exists() {
        return version(engine);
    }
    app.path()
        .resource_dir()
        .ok()
        .and_then(|p| fs::read_to_string(p.join("browser-runtime/version.txt")).ok())
        .unwrap_or_default()
        .trim()
        .into()
}
fn web(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let packaged = app
        .path()
        .resource_dir()
        .map_err(|_| "资源目录不可用。")?
        .join("browser-ui");
    if packaged.join("browser.html").exists() {
        return Ok(packaged);
    }
    if cfg!(debug_assertions) {
        return Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dist"));
    }
    Err("浏览器起始页缺失。".into())
}
fn snapshot(app: &tauri::AppHandle) -> Result<Snapshot, String> {
    let root = root(app)?;
    let settings = workspaces::scope(&root)?;
    let profile = settings.join("profile");
    profile::check_path(&root, &profile)?;
    let machine = config::machine()?;
    // 只读快照：专用端口未设置时显示上次保存的代理，缓存等本地操作仍可用；启动仍走门禁。
    let config = managed_config(app, &settings).or_else(|_| config::load(&settings))?;
    config.validate(&machine)?;
    let engine = engine(app)?;
    let engine_version = packaged_version(app, &engine);
    let running = profile::running(&profile)?;
    let (report, report_received_at) = app
        .state::<Browser>()
        .session
        .lock()
        .unwrap()
        .as_ref()
        .filter(|_| running)
        .map(|x| (x.report.clone(), x.received_at))
        .unwrap_or((None, None));
    let host = app.state::<Browser>().host.lock().unwrap().clone();
    let checks = network::checks(
        &machine,
        &config,
        report.as_ref(),
        host.as_ref(),
        &engine_version,
    );
    let (cache, cache_error) = match profile::scan(&profile) {
        Ok(items) => (items, String::new()),
        Err(error) => (Vec::new(), error),
    };
    Ok(Snapshot {
        ui_language: crate::ui_language::ui_language_read(app.clone()),
        exit_services: &network::SERVICES,
        region: crate::regions::target(&config.language(&machine), &config.timezone(&machine)),
        config,
        machine,
        engine_ready: !engine_version.is_empty(),
        engine_version,
        running,
        profile: profile.to_string_lossy().into(),
        identity_linked: workspaces::database(&root)?.browser_link()?.enabled,
        cache,
        cache_error,
        report,
        report_received_at,
        machine_exit: host,
        checks,
    })
}
async fn run<T: Send + 'static>(
    action: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(action)
        .await
        .map_err(|_| "浏览器操作被中断。".to_string())?
}
#[tauri::command]
pub async fn browser_read(app: tauri::AppHandle) -> Result<Snapshot, String> {
    run(move || {
        let state = app.state::<Browser>();
        let _guard = state.gate.lock().unwrap();
        snapshot(&app)
    })
    .await
}
#[tauri::command]
pub async fn browser_launch(
    app: tauri::AppHandle,
    on_progress: Channel<Progress>,
) -> Result<Snapshot, String> {
    run(move || {
        let _operation = crate::line::exclusive(&app)?;
        let gate = crate::line::browser_gate(&app)?;
        let state = app.state::<Browser>();
        let _guard = state.gate.lock().unwrap();
        launch_locked(&app, gate.endpoint, Some(&gate.check.timezone), &Reporter::channel(Some(on_progress)))
    })
    .await
}


