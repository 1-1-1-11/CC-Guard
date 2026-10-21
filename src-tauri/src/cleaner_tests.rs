// [INPUT]: 隔离临时目录/Windows 子进程 AppData、脱敏配置、清理事务与当前用户官方原生 CLI 版本路径。
// [OUTPUT]: 白名单、文件夹范围、配置保留、组织检索、直接删除、大目录完整统计、读失败拒绝快照、快照保护、真实日志次序与原生版本/可丢弃 Node CLI 进程的识别/终止/隔离验证。
// [POS]: 仅操作测试数据，进程样本由本机编译并握手确认启动，避免重用 Apple 平台二进制，不访问现有 Claude 数据。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::*;
fn junction(source: &Path, target: &Path) {
    assert!(std::process::Command::new("cmd.exe").args(["/D", "/C", "mklink", "/J"]).arg(target.to_string_lossy().replace('/', "\\")).arg(source.to_string_lossy().replace('/', "\\")).output().unwrap().status.success());
}

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let pid = std::process::id();
        let p = std::env::temp_dir().join(format!("rf-cleaner-test-{pid}-{id}-{unique}"));
        fs::create_dir(&p).unwrap();
        crate::windows::fixture_owner(&p);
        Self(canonicalize(p).unwrap())
    }
    fn file(&self, name: &str, value: &str) {
        let p = self.0.join(name);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, value).unwrap();
        for path in p.ancestors().take_while(|path| path.starts_with(&self.0)) { crate::windows::fixture_owner(path); }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn discovers_supported_data_and_preserves_personal_assets() {
    let f = Fixture::new();
    f.file(".claude/cache/data", "cache");
    f.file(".claude/projects/project/session.jsonl", "history");
    f.file(".claude/skills/skill/SKILL.md", "keep");
    f.file(".claude/plugins/plugin", "keep");
    f.file(
        ".claude/settings.json",
        r#"{"permissions":{"allow":["Read"]}}"#,
    );
    f.file(
        ".claude.json",
        r#"{"userID":"old","projects":{"/my-project":{}}}"#,
    );
    f.file("unrelated/.claude.json", r#"{"userID":"unrelated"}"#);
    let plan = collect(&f.0, None).unwrap();
    assert!(plan.items.iter().any(|i| i.group == "cache"));
    assert!(plan.items.iter().any(|i| i.group == "history"));
    assert!(plan.items.iter().any(|i| i.group == "account"));
    assert!(!plan
        .items
        .iter()
        .any(|i| ["skills", "plugins", "settings.json", "unrelated"]
            .iter()
            .any(|name| i.path.contains(name))));
    assert_eq!(
        fs::read_to_string(f.0.join(".claude/skills/skill/SKILL.md")).unwrap(),
        "keep"
    );
}
#[test]
fn ignores_linked_targets() {
    let f = Fixture::new();
    fs::create_dir(f.0.join("outside")).unwrap();
    junction(&f.0.join("outside"), &f.0.join(".claude"));
    assert!(collect(&f.0, None).unwrap().items.is_empty());
}

#[test]
fn account_reset_preserves_projects_and_mcp_and_removes_only_known_account_keys() {
    let original = r#"{"oauthAccount":{"token":"test-only"},"userID":"old","machineID":"old","projects":{"/source":{}},"mcpServers":{"server":{}},"unknown":9}"#;
    let (updated, keys) = config::transform(original, config::EditKind::Account).unwrap();
    let value: serde_json::Value = serde_json::from_str(&updated).unwrap();
    assert!(value.get("oauthAccount").is_none());
    assert!(value.get("userID").is_none());
    assert!(value["projects"]["/source"].is_object());
    assert!(value["mcpServers"]["server"].is_object());
    assert_eq!(value["unknown"], 9);
    assert!(!keys.join(" ").contains("test-only"));
}
#[test]
fn settings_cleanup_preserves_telemetry_hooks_permissions_and_unrelated_secrets() {
    let original = r#"{"env":{"ANTHROPIC_API_KEY":"test-only","HTTPS_PROXY":"test-proxy","DISABLE_TELEMETRY":"1","CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC":"1","OTHER_SECRET":"keep"},"hooks":{"Stop":[]},"permissions":{"allow":["Read"]},"model":"keep"}"#;
    let (updated, _) = config::transform(original, config::EditKind::Settings).unwrap();
    let value: serde_json::Value = serde_json::from_str(&updated).unwrap();
    assert!(value["env"].get("ANTHROPIC_API_KEY").is_none());
    assert_eq!(value["env"]["DISABLE_TELEMETRY"], "1");
    assert_eq!(value["env"]["OTHER_SECRET"], "keep");
    assert_eq!(value["hooks"]["Stop"], serde_json::json!([]));
    assert_eq!(value["model"], "keep");
    assert!(config::transform("{invalid", config::EditKind::Settings).is_err());
    assert!(config::transform(r#"{"env":[]}"#, config::EditKind::Settings).is_err());
}
#[test]
fn shell_and_git_cleanups_do_not_remove_unrelated_commands() {
    let shell = "# HTTPS_PROXY=comment\nexport HTTPS_PROXY='test-only'\nexport ANTHROPIC_API_KEY='test-only'\nexport HTTPS_PROXY=foo; echo keep\nexport KEEP=1\necho proxy\n";
    let (updated, keys) = config::transform(shell, config::EditKind::Shell).unwrap();
    assert_eq!(keys, vec!["ANTHROPIC_API_KEY", "HTTPS_PROXY"]);
    assert!(updated.contains("HTTPS_PROXY=comment"));
    assert!(updated.contains("foo; echo keep"));
    assert!(updated.contains("export KEEP=1"));
    let git = "[http]\n proxy = test-only\n sslVerify = true\n[remote \"origin\"]\n proxy = keep\n";
    let (updated, _) = config::transform(git, config::EditKind::Git).unwrap();
    assert!(!updated.contains("test-only"));
    assert!(updated.contains("sslVerify = true"));
    assert!(updated.contains("proxy = keep"));
}
#[test]
fn organization_search_stays_in_claude_sessions_and_does_not_follow_links() {
    let f = Fixture::new();
    let org = "org-12345678";
    f.file(
        ".claude/projects/a/match.jsonl",
        r#"{"org_id":"org-12345678"}"#,
    );
    f.file(
        ".claude/projects/a/other.jsonl",
        r#"{"org_id":"org-123456789"}"#,
    );
    f.file("source/private.json", r#"{"org_id":"org-12345678"}"#);
    junction(&f.0.join("source"), &f.0.join(".claude/projects/link"));
    let (files, complete) = targets::org_files(&f.0, org).unwrap();
    assert!(complete);
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("match.jsonl"));
    assert!(targets::org_files(&f.0, "../source").is_err());
}

#[test]
fn node_claude_entry_is_identified_and_unrelated_node_is_preserved() {
    struct Children(Vec<std::process::Child>);
    impl Drop for Children {
        fn drop(&mut self) {
            for child in &mut self.0 {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    let f = Fixture::new();
    f.file(
        "node_modules/@anthropic-ai/claude-code/cli.js",
        "setInterval(()=>{},1000)",
    );
    f.file("other.js", "setInterval(()=>{},1000)");
    let mut children = Children(vec![
        std::process::Command::new("node")
            .arg(f.0.join("node_modules/@anthropic-ai/claude-code/cli.js"))
            .spawn()
            .unwrap(),
        std::process::Command::new("node")
            .arg(f.0.join("other.js"))
            .spawn()
            .unwrap(),
    ]);
    let pid = children.0[0].id() as i32;
    let other = children.0[1].id() as i32;
    let mut target = None;
    for _ in 0..20 {
        let current = processes().unwrap();
        assert!(!current.iter().any(|p| p.pid == other));
        target = current.into_iter().find(|p| p.pid == pid);
        if target.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let target = target.expect("必须识别 Node 启动的官方 CLI 入口");
    assert_eq!(target.name, "Claude Code");
    {
        let mut stale = target.clone(); stale.created ^= 1;
        signal_same_process(&stale, 9).unwrap();
        assert!(children.0[0].try_wait().unwrap().is_none());
        let serialized = serde_json::to_string(&target).unwrap();
        assert!(!serialized.contains("created") && !serialized.contains("executable"));
    }
    signal_same_process(&target, 9).unwrap();
    assert!(!children.0[0].wait().unwrap().success());
    assert!(children.0[1].try_wait().unwrap().is_none());
}





#[test]
fn windows_redirected_appdata_remains_whitelisted() {
    // ----- 子进程隔离环境变量，避免并行回归改变真实用户目录 -----
    if std::env::var_os("CC_REDIRECTED_APPDATA_FIXTURE").is_none() {
        let home = Fixture::new();
        let outside = Fixture::new();
        outside.file("roaming/Claude/Cookies", "disposable cookie fixture");
        outside.file("roaming/Claude/local_sessions/record.json", r#"{"org":"org-fixture-123"}"#);
        outside.file("local/claude-cli-nodejs/cache", "disposable cache fixture");
        outside.file("unrelated/data", "keep");
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "cleaner::tests::windows_redirected_appdata_remains_whitelisted", "--nocapture"])
            .env("CC_REDIRECTED_APPDATA_FIXTURE", &outside.0)
            .env("USERPROFILE", &home.0).env("HOME", &home.0)
            .env("APPDATA", outside.0.join("roaming"))
            .env("LOCALAPPDATA", outside.0.join("local"))
            .status().unwrap();
        assert!(status.success());
        assert_eq!(fs::read_to_string(outside.0.join("unrelated/data")).unwrap(), "keep");
        return;
    }
    let root = home().unwrap();
    let outside = PathBuf::from(std::env::var_os("CC_REDIRECTED_APPDATA_FIXTURE").unwrap());
    let scan = current_scan(&root, None).unwrap();
    let cookie = scan.items.iter().find(|item| Path::new(&item.path) == outside.join("roaming/Claude/Cookies")).unwrap();
    let (files, complete) = targets::org_files(&root, "org-fixture-123").unwrap();
    assert!(complete);
    assert_eq!(files, vec![outside.join("roaming/Claude/local_sessions/record.json")]);
    let sources = super::identity::sources(&root, &crate::identities::Policy::default()).unwrap();
    assert!(sources.iter().any(|source| source.path == Path::new(&cookie.path)));
    assert!(sources.iter().all(|source| super::identity::allowed(&root, source)));
}
