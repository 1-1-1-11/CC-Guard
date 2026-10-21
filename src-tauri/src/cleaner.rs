// [INPUT]: Claude 白名单、所选文件夹发现、配置变换、原子写入、官方原生 CLI 版本路径及 Windows 白名单路径边界。
// [OUTPUT]: 全机/文件夹元数据扫描、旧组织发现、实时 Claude 进程检查与结束全部已识别进程；CLI 和身份管理共用互斥锁与精确进程停止函数，身份调用不触发文件扫描。
// [POS]: 白名单与进程共用边界；身份操作由 identities 编排，历史直接清理只编译进测试。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use crate::progress::Reporter;
use serde::Serialize;
use dunce::canonicalize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tauri::Manager;
#[path = "cleaner_config.rs"]
mod config;
#[path = "cleaner_io.rs"]
mod file_io;
#[path = "cleaner_folder.rs"]
mod folder;
#[path = "cleaner_targets.rs"]
mod targets;
#[path = "cleaner_identity.rs"]
pub(crate) mod identity;
#[cfg(test)]
#[path = "cleaner_tests.rs"]
mod tests;
#[derive(Clone)]
enum Operation {
    Delete,
    Edit(config::EditKind),
}
#[derive(Clone, Serialize)]
pub struct Item {
    id: String,
    path: String,
    title: String,
    group: &'static str,
    risk: &'static str,
    impact: String,
    bytes: u64,
    files: usize,
    exact: bool,
    keys: Vec<String>,
}
pub(crate) fn linked(meta: &fs::Metadata) -> bool {
    crate::windows::linked(meta)
}
fn generation() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
#[derive(Clone, Serialize)]
struct Diagnostic {
    title: String,
    detail: String,
    status: &'static str,
}
#[derive(Clone, Serialize, Default)]
pub struct Scan {
    folder: String,
    items: Vec<Item>,
    running: bool,
    processes: Vec<ClaudeProcess>,
    message: String,
    warnings: Vec<String>,
    diagnostics: Vec<Diagnostic>,
    org_id: String,
}
#[derive(Default)]
pub struct Cleaner {
    plan: Mutex<Option<Scan>>,
    active: AtomicBool,
}
impl Cleaner {
    pub(crate) fn enter(&self) -> Result<(), String> {
        if self.active.swap(true, Ordering::SeqCst) {
            Err("另一项操作正在进行，请等它完成后再试。".into())
        } else {
            Ok(())
        }
    }
    pub(crate) fn leave(&self) {
        self.active.store(false, Ordering::SeqCst);
    }
}
fn home() -> Result<PathBuf, String> {
    let var = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
    canonicalize(PathBuf::from(
        var.ok_or("无法定位用户目录。")?,
    ))
    .map_err(|_| "无法读取用户目录。".into())
}
fn add_target(scan: &mut Scan, target: targets::Target, batch: u128) {
    let path = target.path;
    let meta = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                scan.warnings.push(format!("无法检查 {}。", path.display()));
            }
            return;
        }
    };
    let wrong_user = !crate::windows::owned(&path);
    if linked(&meta)
        || wrong_user
        || (!meta.is_dir() && !meta.is_file())
        || canonicalize(&path).ok().as_deref() != Some(path.as_path())
    {
        scan.warnings.push(format!(
            "已跳过链接、其他用户或特殊路径：{}",
            path.display()
        ));
        return;
    }
    let mut keys = Vec::new();
    if let Operation::Edit(kind) = target.operation {
        if !meta.is_file() || meta.len() > 4 * 1024 * 1024 {
            scan.warnings
                .push(format!("配置过大或类型不正确：{}", path.display()));
            return;
        }
        if let Ok(text) = fs::read_to_string(&path) {
            if config::needs_manual_review(&text, kind) {
                scan.warnings.push(format!(
                    "{}：包含复合赋值或外部配置引用，未自动展开或改写这些内容。",
                    path.display()
                ));
            }
        }
        match fs::read_to_string(&path)
            .map_err(|_| "无法读取配置。".to_string())
            .and_then(|s| config::transform(&s, kind))
        {
            Ok((_, names)) => {
                if names.is_empty() {
                    return;
                }
                keys = names;
            }
            Err(e) => {
                scan.warnings.push(format!("{}：{e}", path.display()));
                return;
            }
        }
    }
    let (bytes, files, _) = match targets::measure(&path) {
        Ok(measured) => measured,
        Err(error) => { scan.warnings.push(error); return; }
    };
    scan.items.push(Item {
        id: format!("{batch}-{}", scan.items.len()),
        path: path.to_string_lossy().into_owned(),
        title: target.title,
        group: target.group,
        risk: target.risk,
        impact: target.impact,
        bytes,
        files,
        exact: true,
        keys,
    });
}
fn collect(root: &Path, org: Option<&str>) -> Result<Scan, String> {
    let batch = generation();
    let mut scan = Scan::default();
    for target in targets::definitions(root) {
        add_target(&mut scan, target, batch);
    }
    if let Some(org) = org.filter(|s| !s.is_empty()) {
        let (matches, complete) = targets::org_files(root, org)?;
        scan.org_id = org.into();
        for path in matches {
            if scan.items.iter().any(|i| i.path == path.to_string_lossy()) {
                continue;
            }
            add_target(
                &mut scan,
                targets::Target {
                    title: "旧组织会话文件".into(),
                    path,
                    group: "organization",
                    risk: "history",
                    impact: "文件正文匹配所填组织编号；请核对路径，只清理确认不再需要的旧会话。"
                        .into(),
                    operation: Operation::Delete,
                },
                batch,
            );
        }
        if !complete {
            scan.warnings
                .push("组织检索达到时间或条目上限；未扫描全部文件。".into());
        }
    }
    scan.message = format!("已发现 {} 个可选项目。", scan.items.len());
    Ok(scan)
}
fn diagnostics(scan: &mut Scan) {
    let names: Vec<_> = std::env::vars_os()
        .filter_map(|(key, _)| key.into_string().ok())
        .filter(|key| {
            matches!(
                key.to_ascii_uppercase().as_str(),
                "HTTP_PROXY"
                    | "HTTPS_PROXY"
                    | "ALL_PROXY"
                    | "NO_PROXY"
                    | "CLAUDE_CODE_OAUTH_TOKEN"
                    | "CLAUDE_CONFIG_DIR"
            ) || key.starts_with("ANTHROPIC_")
        })
        .collect();
    scan.diagnostics.push(Diagnostic {
        title: "当前应用环境".into(),
        detail: if names.is_empty() {
            "未发现代理或认证变量。终端已启动的会话需单独检查。".into()
        } else {
            format!(
                "发现变量：{}。清理启动文件后，请重开终端。",
                names.join("、")
            )
        },
        status: if names.is_empty() { "ok" } else { "review" },
    });
    if std::env::var_os("CLAUDE_CONFIG_DIR").is_some() {
        scan.warnings.push(
            "检测到自定义 CLAUDE_CONFIG_DIR；本次仅检查默认目录，自定义目录未纳入清理。".into(),
        );
    }
    {
        let enabled = [
            "ALL_PROXY",
            "all_proxy",
            "HTTPS_PROXY",
            "https_proxy",
            "HTTP_PROXY",
            "http_proxy",
        ]
        .iter()
        .any(|key| std::env::var_os(key).is_some());
        scan.diagnostics.push(Diagnostic {
            title: "代理环境".into(),
            detail: if enabled {
                "检测到代理环境变量；桌面代理设置未独立核验，请验证实际出口。".into()
            } else {
                "未发现代理环境变量；TUN 是推荐的网络环境。".into()
            },
            status: if enabled { "review" } else { "info" },
        });
    }
    let timezone = crate::windows::timezone();
    scan.diagnostics.push(Diagnostic {
        title: "设备时区".into(),
        detail: format!("{timezone} · 可在电脑设置中修改。"),
        status: "info",
    });
    scan.diagnostics.push(Diagnostic {
        title: "保留个人资产".into(),
        detail: "skills、plugins、hooks、commands、CLAUDE.md、MCP、权限、模型及遥测设置保持不变。"
            .into(),
        status: "ok",
    });
}
#[derive(Clone, Serialize)]
pub struct ClaudeProcess {
    pub pid: i32,
    pub name: String,
    #[serde(skip)]
    #[allow(dead_code)]
    pub executable: String,
    #[serde(skip)]
    pub created: u64,
}
fn processes() -> Result<Vec<ClaudeProcess>, String> {
    crate::windows::claude_processes()
}
fn running() -> Result<bool, String> {
    Ok(!processes()?.is_empty())
}
pub(crate) fn cli_running() -> Result<bool, String> {
    Ok(processes()?.iter().any(|process| {
        matches!(
            process.name.as_str(),
            "Claude Code" | "claude" | "claude.exe" | "claude-code" | "claude-code-vm"
        )
    }))
}
fn signal_same_process(target: &ClaudeProcess, _signal: i32) -> Result<(), String> {
    crate::windows::kill_process(target)
}

fn current_scan(root: &Path, org: Option<&str>) -> Result<Scan, String> {
    let mut scan = collect(root, org)?;
    scan.processes = processes()?;
    scan.running = !scan.processes.is_empty();
    diagnostics(&mut scan);
    scan.message = format!("已发现 {} 个可选项目。", scan.items.len());
    Ok(scan)
}
fn scan_scope(
    folder: Option<&str>,
    org: Option<&str>,
    reporter: &Reporter,
) -> Result<Scan, String> {
    if let Some(path) = folder.filter(|p| !p.is_empty()) {
        let mut scan = folder::collect(Path::new(path), reporter)?;
        scan.processes = processes()?;
        scan.running = !scan.processes.is_empty();
        scan.diagnostics.push(Diagnostic { title: "文件夹范围".into(), detail: "仅检查所选文件夹内的 Claude 会话、缓存与既有备份。项目文档、设置、凭据、技能和插件保留；不检查目录外钥匙串或全局会话。".into(), status: "info" });
        Ok(scan)
    } else {
        current_scan(&home()?, org)
    }
}
pub(crate) fn stop_processes() -> Result<usize, String> {
    let mut targets = std::collections::HashSet::new();
    // 主进程退出期间辅助进程可能重生；每轮重新识别，最多三轮，不扩大到其他 Node/MCP/终端。
    for _ in 0..3 {
        let current = processes()?;
        if current.is_empty() {
            break;
        }
        for target in &current {
            targets.insert(target.pid);
            signal_same_process(target, 15)?;
        }
        std::thread::sleep(std::time::Duration::from_millis(800));
        for target in processes()? {
            targets.insert(target.pid);
            signal_same_process(&target, 9)?;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    if running()? { return Err("仍有 Claude 进程在运行。".into()); }
    Ok(targets.len())
}
#[tauri::command]
pub async fn cleaner_stop(app: tauri::AppHandle) -> Result<Scan, String> {
    let state = app.state::<Cleaner>();
    state.enter()?;
    let worker = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let count = stop_processes()?;
        let plan = worker.state::<Cleaner>().plan.lock().unwrap().clone();
        let mut scan = scan_scope(
            plan.as_ref().map(|p| p.folder.as_str()),
            plan.as_ref().map(|p| p.org_id.as_str()),
            &Reporter::channel(None),
        )?;
        scan.message = if scan.running {
            "仍有 Claude 进程在运行。".into()
        } else {
            format!("已结束 {} 个 Claude 进程，可以清理。", count)
        };
        *worker.state::<Cleaner>().plan.lock().unwrap() = Some(scan.clone());
        Ok(scan)
    })
    .await
    .map_err(|_| "结束进程被中断，请刷新。".to_string())
    .and_then(|r| r);
    state.active.store(false, Ordering::SeqCst);
    result
}
