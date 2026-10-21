// [INPUT]: 用户 mihomo 的 controller 地址与 secret、指定节点名、Claude 域名后缀与进程名名单。
// [OUTPUT]: 一次 /connections 快照的漏网核对：Claude 流量（域名或进程命中）的链路里没有指定的节点或代理组即拦住；漏网记录里的 node 是实际节点 chains[0]。
// [POS]: 连接核对；只读 /configs 与 /connections，读不到、格式不对都不是通过。判卷台 tests/mihomo-harness/connections.py。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::{get, Fetch, Verdict};
use serde::Serialize;
use serde_json::Value;

pub struct Input {
    pub controller: String,
    pub secret: String,
    pub node: String,
    pub claude_hosts: Vec<String>,
    pub claude_processes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Leak {
    pub host: String,
    pub process: String,
    pub inbound: String,
    pub node: String,
}

#[derive(Debug, Serialize)]
pub struct Outcome {
    pub verdict: Verdict,
    pub label: &'static str,
    pub reason: String,
    pub total: usize,
    pub claude: usize,
    pub leaks: Vec<Leak>,
    pub process_matching: bool,
    pub checked_at: u64,
}

fn outcome(verdict: Verdict, reason: impl Into<String>) -> Outcome {
    Outcome {
        verdict,
        label: super::label(verdict),
        reason: reason.into(),
        total: 0,
        claude: 0,
        leaks: Vec::new(),
        process_matching: false,
        checked_at: super::now_ms(),
    }
}

fn host_key(value: &str) -> String {
    value.trim().trim_end_matches('.').trim_start_matches('.').to_lowercase()
}

/// 按域名后缀认 Claude：等于后缀或以「.后缀」结尾，不分大小写。
pub fn host_matches(host: &str, suffixes: &[String]) -> bool {
    let host = host_key(host);
    !host.is_empty()
        && suffixes.iter().map(|s| host_key(s)).filter(|s| !s.is_empty()).any(|s| host == s || host.ends_with(&format!(".{s}")))
}

/// 按进程名认 Claude：取路径末段，不分大小写整名相等，不做包含或前缀匹配。
pub fn process_matches(process: &str, names: &[String]) -> bool {
    let name = process.trim().rsplit(['\\', '/']).next().unwrap_or("").to_lowercase();
    !name.is_empty() && names.iter().any(|n| n.trim().to_lowercase() == name)
}

/// 纯判定：快照里每条连接是否属于 Claude、是否走了指定节点。
pub fn judge(data: &Value, input: &Input, process_matching: bool) -> Outcome {
    let list = match &data["connections"] {
        Value::Null => Vec::new(),
        Value::Array(items) => items.clone(),
        _ => return outcome(Verdict::Unknown, "mihomo 连接列表格式不对，无法确认。"),
    };
    let node = input.node.trim();
    let mut claude = 0;
    let mut leaks = Vec::new();
    for item in &list {
        let meta = &item["metadata"];
        if !item.is_object() || !meta.is_object() {
            return outcome(Verdict::Unknown, "mihomo 连接记录格式不对，无法确认。");
        }
        let text = |key: &str| meta[key].as_str().unwrap_or("").to_string();
        let (host, sniffed, process, path) = (text("host"), text("sniffHost"), text("process"), text("processPath"));
        let is_claude = host_matches(&host, &input.claude_hosts)
            || host_matches(&sniffed, &input.claude_hosts)
            || process_matches(&process, &input.claude_processes)
            || process_matches(&path, &input.claude_processes);
        if !is_claude {
            continue;
        }
        claude += 1;
        // 指定的可以是节点或代理组：只要它出现在这条连接的链路里就算走对（fallback 组内切换不误报）。
        // 读不到链路也算漏网：宁可误拦。实际出口 IP 由线路检查另行核对。
        let chain: Vec<&str> = item["chains"].as_array().map(|c| c.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
        let actual = chain.first().copied().unwrap_or("");
        if !chain.contains(&node) {
            let shown = if host.is_empty() { sniffed } else { host };
            let shown = if shown.is_empty() { meta["destinationIP"].as_str().unwrap_or("").to_string() } else { shown };
            leaks.push(Leak { host: shown, process, inbound: text("inboundName"), node: actual.into() });
        }
    }
    let total = list.len();
    let scope = if process_matching { "" } else { "（Clash 未开 find-process-mode: always，只按域名核对）" };
    let (verdict, reason) = if !leaks.is_empty() {
        let mut names: Vec<String> = Vec::new();
        for leak in &leaks {
            let name = if leak.host.is_empty() { leak.process.clone() } else { leak.host.clone() };
            if !names.contains(&name) {
                names.push(name);
            }
        }
        (Verdict::Block, format!("发现 {} 条 Claude 流量没走指定节点 {node}：{}，已拦住{scope}。", leaks.len(), names.join("、")))
    } else if total == 0 {
        (Verdict::Pass, format!("此刻没有连接，没有发现漏网{scope}。"))
    } else if claude == 0 {
        (Verdict::Pass, format!("此刻有 {total} 条连接，没有 Claude 流量，没有发现漏网{scope}。"))
    } else {
        (Verdict::Pass, format!("{claude} 条 Claude 连接都走指定节点 {node}{scope}。"))
    };
    Outcome { verdict, label: super::label(verdict), reason, total, claude, leaks, process_matching, checked_at: super::now_ms() }
}

pub async fn check(input: &Input) -> Outcome {
    if input.controller.trim().is_empty() {
        return outcome(Verdict::Unknown, "还没设置 mihomo controller 地址，无法核对连接。");
    }
    if input.node.trim().is_empty() {
        return outcome(Verdict::Unknown, "还没设定指定节点，无法核对连接。");
    }
    if !input.claude_hosts.iter().any(|h| !host_key(h).is_empty()) {
        return outcome(Verdict::Unknown, "Claude 域名名单为空，无法核对连接。");
    }
    let process_matching = match get(&input.controller, &input.secret, "/configs", 262_144).await {
        Fetch::Ok(data) => data["find-process-mode"].as_str() == Some("always"),
        Fetch::Unauthorized => return outcome(Verdict::Block, "mihomo 拒绝了 secret，读不到连接，已拦住。"),
        Fetch::Failed(e) => return outcome(Verdict::Unknown, format!("{e}无法确认。")),
    };
    match get(&input.controller, &input.secret, "/connections", 16 * 1024 * 1024).await {
        Fetch::Ok(data) if data.is_object() => judge(&data, input, process_matching),
        Fetch::Ok(_) => outcome(Verdict::Unknown, "mihomo 连接列表格式不对，无法确认。"),
        Fetch::Unauthorized => outcome(Verdict::Block, "mihomo 拒绝了 secret，读不到连接，已拦住。"),
        Fetch::Failed(e) => outcome(Verdict::Unknown, format!("{e}无法确认。")),
    }
}
