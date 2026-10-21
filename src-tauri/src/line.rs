// [INPUT]: 应用数据目录 settings.json（线路设置与 CLI 工作目录）、用户 mihomo 控制接口、原生文件夹选择框。
// [OUTPUT]: src/contract.ts 的 settings_read/write、line_check、line_connections、line_nodes、launch、workspace_choose；
//           托盘、身份复查与浏览器共用的同步门禁 gate()。
// [POS]: 启动门禁的唯一入口：每次启动重新做线路检查，只有「通过」才启动；secret 只写不读，从不返回界面。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use crate::mihomo::{self, connections, Outcome, Verdict};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::Manager;

pub const DEFAULT_HOSTS: [&str; 3] = ["anthropic.com", "claude.ai", "claude.com"];
pub const DEFAULT_PROCESSES: [&str; 1] = ["claude.exe"];
/// 线路检查的出口查询服务。静态 IP 常被 ipapi.co 限流（HTTP 429）、ipwho.is 也可能连不上，
/// 所以放三个独立服务，成功的至少 2 个且一致才算数（2026-10-21 真机实测）。
pub const LINE_SERVICES: [&str; 3] =
    ["https://ifconfig.co/json", "https://www.cloudflare.com/cdn-cgi/trace", "https://ipwho.is/"];
/// 桌面版（MSIX）的执行别名：允许带参数启动，从而用 Chromium 的 --proxy-server 固定到专用端口
/// （2026-10-21 真机核实：别名指向 WindowsApps 里的 Claude.exe；shell:AppsFolder 方式不能带参数）。
const DESKTOP_ALIAS: &str = r"Microsoft\WindowsApps\claude-desktop.exe";

/// 桌面版启动命令：只带一个代理开关，且必须是专用端口。
pub(crate) fn desktop_command(alias: &Path, endpoint: &str) -> Command {
    let mut command = Command::new(alias);
    command.arg(format!("--proxy-server={endpoint}"));
    command
}

/// 写设置时互斥，避免两次写入互相覆盖。
#[derive(Default)]
pub struct Line(Mutex<()>);

// ----- 设置：磁盘格式（含 secret）与界面格式（只有 secret_set）分开 -----
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Stored {
    pub line: StoredLine,
    pub workspace: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StoredLine {
    pub controller: String,
    pub secret: String,
    pub port: u16,
    pub node: String,
    pub expected_ip: String,
    pub services: Vec<String>,
    pub claude_hosts: Vec<String>,
    pub claude_processes: Vec<String>,
}
impl Default for Stored {
    fn default() -> Self {
        Self { line: StoredLine::default(), workspace: String::new() }
    }
}
impl Default for StoredLine {
    fn default() -> Self {
        Self {
            controller: String::new(),
            secret: String::new(),
            port: 0,
            node: String::new(),
            expected_ip: String::new(),
            services: LINE_SERVICES.iter().map(|s| s.to_string()).collect(),
            claude_hosts: DEFAULT_HOSTS.iter().map(|s| s.to_string()).collect(),
            claude_processes: DEFAULT_PROCESSES.iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LineSettings {
    controller: String,
    secret_set: bool,
    port: u16,
    node: String,
    expected_ip: String,
    services: Vec<String>,
    claude_hosts: Vec<String>,
    claude_processes: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct Settings {
    line: LineSettings,
    workspace: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinePatch {
    controller: Option<String>,
    secret: Option<String>,
    port: Option<u16>,
    node: Option<String>,
    expected_ip: Option<String>,
    services: Option<Vec<String>>,
    claude_hosts: Option<Vec<String>>,
    claude_processes: Option<Vec<String>>,
}
/// 没有 workspace 字段：工作目录只能经 workspace_choose 原生对话框修改，带了就整体拒绝。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsPatch {
    line: Option<LinePatch>,
}

fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map_err(|_| "应用目录不可用。".into())
}

pub fn load_from(dir: &Path) -> Result<Stored, String> {
    let file = dir.join("settings.json");
    match std::fs::read(&file) {
        Ok(bytes) => serde_json::from_slice::<Stored>(&bytes)
            .map(|mut stored| {
                // 早期版本存下的默认服务（浏览器那组）在静态 IP 上常不可用，读到原样默认值就换成线路检查的默认值。
                if stored.line.services == crate::browser::network::SERVICES {
                    stored.line.services = LINE_SERVICES.iter().map(|s| s.to_string()).collect();
                }
                stored
            })
            .map_err(|_| format!("设置文件 {} 已损坏，未做任何改动；请删除它后重新设置。", file.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Stored::default()),
        Err(_) => Err("设置文件读取失败。".into()),
    }
}

fn save_to(dir: &Path, stored: &Stored) -> Result<(), String> {
    crate::identities::private::directory(dir)?;
    let bytes = serde_json::to_vec_pretty(stored).map_err(|_| "设置编码失败。")?;
    crate::identities::private::write(&dir.join("settings.json"), &bytes)
}

pub fn load(app: &tauri::AppHandle) -> Result<Stored, String> {
    load_from(&data_dir(app)?)
}

fn usable(path: &Path) -> bool {
    path.is_absolute() && path.is_dir()
}

/// CLI 工作目录：设置里的目录仍可用才用，否则回到用户主目录；每次使用都重新核对。
fn workspace_of(app: &tauri::AppHandle, stored: &Stored) -> Result<PathBuf, String> {
    let saved = PathBuf::from(&stored.workspace);
    if !stored.workspace.is_empty() && usable(&saved) {
        return Ok(saved);
    }
    // 旧版把工作目录单独存在 cli-workspace.json；迁移过来的数据照样认。
    let legacy = data_dir(app)
        .ok()
        .and_then(|dir| std::fs::read(dir.join("cli-workspace.json")).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|value| value["path"].as_str().map(PathBuf::from))
        .filter(|path| stored.workspace.is_empty() && usable(path));
    if let Some(path) = legacy {
        return Ok(path);
    }
    app.path().home_dir().map_err(|_| "无法定位用户主目录。".into())
}

pub fn view(stored: &Stored, workspace: &Path) -> Settings {
    let line = &stored.line;
    Settings {
        line: LineSettings {
            controller: line.controller.clone(),
            secret_set: !line.secret.is_empty(),
            port: line.port,
            node: line.node.clone(),
            expected_ip: line.expected_ip.clone(),
            services: line.services.clone(),
            claude_hosts: line.claude_hosts.clone(),
            claude_processes: line.claude_processes.clone(),
        },
        workspace: workspace.to_string_lossy().into_owned(),
    }
}

fn text(value: String, field: &str, max: usize) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(format!("{field} 格式无效。"));
    }
    Ok(value)
}

fn list(values: Vec<String>, field: &str, check: impl Fn(&str) -> bool) -> Result<Vec<String>, String> {
    if values.len() > 16 {
        return Err(format!("{field} 最多 16 项。"));
    }
    let mut kept: Vec<String> = Vec::new();
    for value in values {
        let value = text(value, field, 253)?;
        if value.is_empty() || !check(&value) {
            return Err(format!("{field} 里有无效项：{value}"));
        }
        if !kept.iter().any(|k| k.eq_ignore_ascii_case(&value)) {
            kept.push(value);
        }
    }
    Ok(kept)
}

/// 合并补丁：只改带了的字段；每个字段先校验，任一无效整体拒绝、不落盘。
pub fn apply(mut stored: Stored, patch: SettingsPatch) -> Result<Stored, String> {
    let Some(p) = patch.line else { return Ok(stored) };
    let line = &mut stored.line;
    if let Some(v) = p.controller {
        let v = text(v, "controller", 256)?;
        if !v.is_empty() && mihomo::controller_valid(&v).is_err() {
            return Err("controller 地址格式无效，应为 127.0.0.1:9097 这样的地址。".into());
        }
        line.controller = v;
    }
    if let Some(v) = p.secret {
        if v.len() > 1024 || v.chars().any(char::is_control) {
            return Err("secret 格式无效。".into());
        }
        line.secret = v;
    }
    if let Some(v) = p.port {
        line.port = v;
    }
    if let Some(v) = p.node {
        line.node = text(v, "节点名", 256)?;
    }
    if let Some(v) = p.expected_ip {
        let v = text(v, "期望出口 IP", 64)?;
        if !v.is_empty() && v.parse::<std::net::IpAddr>().is_err() {
            return Err("期望出口 IP 格式无效。".into());
        }
        line.expected_ip = v;
    }
    if let Some(v) = p.services {
        line.services = list(v, "出口查询服务", |s| {
            reqwest::Url::parse(s).is_ok_and(|u| matches!(u.scheme(), "http" | "https") && u.host_str().is_some())
        })?;
    }
    if let Some(v) = p.claude_hosts {
        line.claude_hosts = list(v, "Claude 域名", |s| {
            s.trim_matches('.').split('.').all(|label| {
                !label.is_empty() && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
        })?
        .into_iter()
        .map(|s| s.trim_matches('.').to_ascii_lowercase())
        .collect();
    }
    if let Some(v) = p.claude_processes {
        line.claude_processes = list(v, "Claude 进程名", |s| !s.contains(['\\', '/', ':', '*', '?']))?;
    }
    Ok(stored)
}

// ----- 线路检查与门禁 -----
fn check_input(line: &StoredLine) -> mihomo::Input {
    mihomo::Input {
        controller: line.controller.clone(),
        secret: line.secret.clone(),
        port: line.port,
        expected_ip: line.expected_ip.clone(),
        services: line.services.clone(),
    }
}

fn connections_input(line: &StoredLine) -> connections::Input {
    connections::Input {
        controller: line.controller.clone(),
        secret: line.secret.clone(),
        node: line.node.clone(),
        claude_hosts: line.claude_hosts.clone(),
        claude_processes: line.claude_processes.clone(),
    }
}

pub fn endpoint(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

/// 已保存的专用端口（不做检查）：只给浏览器快照显示与参数文件用，启动一律走 gate()。
pub fn saved_endpoint(app: &tauri::AppHandle) -> Result<String, String> {
    match load(app)?.line.port {
        0 => Err("还没设置 Claude 专用端口。".into()),
        port => Ok(endpoint(port)),
    }
}

/// 设置读不出来时也给一个有原因的「无法确认」，绝不当成通过。
fn unconfirmed(reason: String) -> Outcome {
    mihomo::unconfirmed(reason)
}

async fn check_stored(stored: Result<Stored, String>) -> Outcome {
    match stored {
        Ok(stored) => mihomo::check(&check_input(&stored.line)).await,
        Err(e) => unconfirmed(e),
    }
}

/// 通过门禁的线路：专用端口地址 + 本次检查结果（时区跟随实际出口）。
pub struct Gate {
    pub endpoint: String,
    pub check: Outcome,
}

/// 同步门禁：每次都重新检查；只有「通过」才给出端口。调用方须在非异步线程里调用。
pub fn gate(app: &tauri::AppHandle) -> Result<Gate, Outcome> {
    let stored = load(app);
    let port = stored.as_ref().map(|s| s.line.port).unwrap_or(0);
    let check = tauri::async_runtime::block_on(check_stored(stored));
    passed(check, port)
}

pub fn passed(check: Outcome, port: u16) -> Result<Gate, Outcome> {
    if check.verdict == Verdict::Pass && port != 0 {
        Ok(Gate { endpoint: endpoint(port), check })
    } else {
        Err(check)
    }
}

/// 浏览器各启动路径共用：身份恢复未完成或线路未通过都不放行，错误带原因。
pub fn browser_gate(app: &tauri::AppHandle) -> Result<Gate, String> {
    crate::identities::ensure_settled(app)?;
    gate(app).map_err(|check| format!("线路检查未通过，浏览器未启动：{}", check.reason))
}

// ----- 共享操作锁：与身份事务、CLI 安装管理互斥 -----
pub struct Operation(tauri::AppHandle);
pub fn exclusive(app: &tauri::AppHandle) -> Result<Operation, String> {
    app.state::<crate::cleaner::Cleaner>().enter()?;
    Ok(Operation(app.clone()))
}
impl Drop for Operation {
    fn drop(&mut self) {
        self.0.state::<crate::cleaner::Cleaner>().leave();
    }
}
/// 其他程序交来的链接排在正在进行的操作之后。
pub fn exclusive_wait(app: &tauri::AppHandle, limit: Duration) -> Result<Operation, String> {
    let started = Instant::now();
    loop {
        match exclusive(app) {
            Err(_) if started.elapsed() < limit => std::thread::sleep(Duration::from_millis(150)),
            result => return result,
        }
    }
}

// ----- 启动 -----
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Entry {
    Cli,
    Desktop,
    Web,
}

#[derive(Debug, Serialize)]
pub struct LaunchResult {
    pub launched: bool,
    pub reason: String,
    pub check: Outcome,
}

/// 纯门禁：检查没通过就不调用 start；通过才启动，启动失败如实返回错误。
pub fn decide_launch(
    check: Outcome,
    port: u16,
    start: impl FnOnce(&Gate) -> Result<(), String>,
) -> Result<LaunchResult, String> {
    match passed(check, port) {
        Err(check) => Ok(LaunchResult { launched: false, reason: format!("线路检查未通过，已停止启动：{}", check.reason), check }),
        Ok(gate) => {
            start(&gate)?;
            Ok(LaunchResult { launched: true, reason: "线路检查通过，已启动。".into(), check: gate.check })
        }
    }
}

/// CLI 子进程：只在它自己的环境里指向专用端口，并去掉可能抢先生效的 ALL_PROXY
/// （Windows 环境变量名不分大小写，覆盖大写即覆盖小写）。
pub fn cli_command(workspace: &Path, endpoint: &str) -> Command {
    let mut command = Command::new("cmd.exe");
    command.args(["/K", "claude"]).current_dir(workspace);
    command.env_remove("ALL_PROXY");
    command.env("HTTP_PROXY", endpoint).env("HTTPS_PROXY", endpoint).env("NO_PROXY", "localhost,127.0.0.1");
    command
}

fn start(app: &tauri::AppHandle, entry: Entry, stored: &Stored, gate: &Gate) -> Result<(), String> {
    match entry {
        Entry::Cli => {
            let workspace = workspace_of(app, stored)?;
            let mut command = cli_command(&workspace, &gate.endpoint);
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x0000_0010); // CREATE_NEW_CONSOLE
            }
            command.spawn().map(drop).map_err(|_| "无法打开 Claude CLI 终端。".into())
        }
        Entry::Desktop => {
            // 桌面版是单实例：已在运行时再启动只会唤起旧窗口，新的代理开关不生效。
            if crate::windows::claude_processes()?.iter().any(|p| p.name == "Claude Desktop") {
                return Err("Claude 桌面版已经在运行，再启动不会带上专用端口。请先在托盘里完全退出桌面版，再从 CC-Guard 启动。".into());
            }
            let alias = std::env::var_os("LOCALAPPDATA")
                .map(|dir| PathBuf::from(dir).join(DESKTOP_ALIAS))
                .filter(|path| std::fs::symlink_metadata(path).is_ok())
                .ok_or("找不到桌面版的启动别名 claude-desktop.exe，无法固定到专用端口，没有启动。请确认 Claude 桌面版是从官方安装的。")?;
            let mut command = desktop_command(&alias, &gate.endpoint);
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x0000_0008 | 0x0000_0200); // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
            }
            command.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
            command.spawn().map(drop).map_err(|_| "无法启动 Claude 桌面客户端。".into())
        }
        Entry::Web => crate::browser::open_gated(app, gate.endpoint.clone(), Some(&gate.check.timezone), &[]),
    }
}

/// 首页与托盘共用的唯一启动路径。
pub fn launch_blocking(app: &tauri::AppHandle, entry: Entry) -> Result<LaunchResult, String> {
    let _operation = exclusive(app)?;
    crate::identities::ensure_settled(app)?;
    let stored = load(app)?;
    let check = tauri::async_runtime::block_on(mihomo::check(&check_input(&stored.line)));
    decide_launch(check, stored.line.port, |gate| start(app, entry, &stored, gate))
}

// ----- 命令 -----
#[tauri::command]
pub async fn settings_read(app: tauri::AppHandle) -> Result<Settings, String> {
    let stored = load(&app)?;
    Ok(view(&stored, &workspace_of(&app, &stored)?))
}

#[tauri::command]
pub async fn settings_write(app: tauri::AppHandle, patch: SettingsPatch) -> Result<Settings, String> {
    let state = app.state::<Line>();
    let _lock = state.0.lock().map_err(|_| "设置暂不可用。")?;
    let dir = data_dir(&app)?;
    let stored = apply(load_from(&dir)?, patch)?;
    save_to(&dir, &stored)?;
    Ok(view(&stored, &workspace_of(&app, &stored)?))
}

#[tauri::command]
pub async fn line_check(app: tauri::AppHandle) -> Result<Outcome, String> {
    Ok(check_stored(load(&app)).await)
}

#[tauri::command]
pub async fn line_connections(app: tauri::AppHandle) -> Result<connections::Outcome, String> {
    let stored = load(&app)?;
    Ok(connections::check(&connections_input(&stored.line)).await)
}

#[tauri::command]
pub async fn line_nodes(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let stored = load(&app)?;
    mihomo::nodes(&stored.line.controller, &stored.line.secret).await
}

#[tauri::command]
pub async fn launch(app: tauri::AppHandle, entry: Entry) -> Result<LaunchResult, String> {
    tauri::async_runtime::spawn_blocking(move || launch_blocking(&app, entry))
        .await
        .map_err(|_| "启动中断。")?
}

fn pick_folder(owner: isize, start: PathBuf) -> Result<Option<PathBuf>, String> {
    use windows::core::{HRESULT, HSTRING};
    use windows::Win32::{Foundation::{ERROR_CANCELLED, HWND}, System::Com::*, UI::Shell::*};
    unsafe {
        let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).is_ok();
        let result = (|| -> windows::core::Result<Option<PathBuf>> {
            let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
            dialog.SetOptions(dialog.GetOptions()? | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST | FOS_NOCHANGEDIR)?;
            if let Ok(item) = SHCreateItemFromParsingName::<_, _, IShellItem>(&HSTRING::from(start.as_os_str()), None) {
                let _ = dialog.SetFolder(&item);
            }
            if let Err(error) = dialog.Show(Some(HWND(owner as _))) {
                return if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) { Ok(None) } else { Err(error) };
            }
            let name = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
            let path = name.to_string().map(PathBuf::from);
            CoTaskMemFree(Some(name.0 as _));
            Ok(path.ok())
        })();
        if initialized {
            CoUninitialize();
        }
        result.map_err(|_| "无法打开文件夹选择窗口。".into())
    }
}

/// 打开系统文件夹选择框；路径只来自原生对话框，WebView 从不提供路径。
#[tauri::command]
pub async fn workspace_choose(app: tauri::AppHandle) -> Result<Option<String>, String> {
    {
        let owner = app.get_webview_window("main").and_then(|w| w.hwnd().ok()).map_or(0, |hwnd| hwnd.0 as isize);
        let start = workspace_of(&app, &load(&app)?)?;
        let (send, receive) = tokio::sync::oneshot::channel();
        // COM 对话框需要自己的单线程单元。
        std::thread::spawn(move || {
            let _ = send.send(pick_folder(owner, start));
        });
        let Some(chosen) = receive.await.map_err(|_| "文件夹选择中断。")?? else { return Ok(None) };
        let chosen = dunce::canonicalize(&chosen).map_err(|_| "所选文件夹不可用。")?;
        if !usable(&chosen) {
            return Err("所选文件夹不可用。".into());
        }
        let state = app.state::<Line>();
        let _lock = state.0.lock().map_err(|_| "设置暂不可用。")?;
        let dir = data_dir(&app)?;
        let mut stored = load_from(&dir)?;
        stored.workspace = chosen.to_string_lossy().into_owned();
        save_to(&dir, &stored)?;
        Ok(Some(stored.workspace))
    }
}

#[cfg(test)]
#[path = "line_tests.rs"]
mod tests;
