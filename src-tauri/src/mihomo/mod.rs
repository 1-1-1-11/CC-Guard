// [INPUT]: 用户 mihomo 的 controller 地址与 secret、Claude 专用入站端口、期望出口 IP、≥2 个出口查询服务。
// [OUTPUT]: 通过 / 拦住 / 无法确认 + 一句中文原因 + 读到的分流模式与实际出口 IP/国家/时区；connections 子模块做漏网核对。
// [POS]: Claude 专用线路检查（line_check 与启动门禁）；只读 /configs、逐个服务单独经专用端口探测，读不到、超时、格式不对都不是通过。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use crate::browser::network::{probe_urls, Exit};
use serde::Serialize;
use serde_json::Value;
use std::net::IpAddr;
use std::time::Duration;
pub mod connections;
#[cfg(test)]
mod live;
#[cfg(test)]
mod tests;

pub struct Input {
    pub controller: String,
    pub secret: String,
    pub port: u16,
    pub expected_ip: String,
    pub services: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Pass,
    Block,
    Unknown,
}

/// 与 src/contract.ts 的 LineCheck 一致。
#[derive(Clone, Debug, Serialize)]
pub struct Outcome {
    pub verdict: Verdict,
    pub label: &'static str,
    pub reason: String,
    pub mode: String,
    pub ip: String,
    pub country: String,
    pub timezone: String,
    pub checked_at: u64,
}

pub(crate) fn label(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Pass => "通过",
        Verdict::Block => "拦住",
        Verdict::Unknown => "无法确认",
    }
}

pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn outcome(verdict: Verdict, reason: impl Into<String>, mode: &str, ip: &str) -> Outcome {
    Outcome {
        verdict,
        label: label(verdict),
        reason: reason.into(),
        mode: mode.into(),
        ip: ip.into(),
        country: String::new(),
        timezone: String::new(),
        checked_at: now_ms(),
    }
}

/// 带上观测到的国家与时区（只来自实际出口，不来自设置）。
fn observed(mut result: Outcome, exit: &Exit) -> Outcome {
    result.country = exit.country.clone();
    result.timezone = exit.timezone.clone();
    result
}

/// /configs 里读到的分流模式与全部入站端口（0 表示未开启）。
#[derive(Debug, PartialEq)]
pub struct Configs {
    pub mode: String,
    pub inbound_ports: Vec<u64>,
}

const INBOUND_KEYS: [&str; 5] = ["port", "socks-port", "mixed-port", "redir-port", "tproxy-port"];

pub fn parse_configs(data: &Value) -> Result<Configs, String> {
    let mode = data["mode"]
        .as_str()
        .filter(|m| !m.is_empty())
        .ok_or("mihomo 配置里没有分流模式。")?;
    let mut inbound_ports = Vec::new();
    for key in INBOUND_KEYS {
        let port = data[key]
            .as_u64()
            .filter(|p| *p <= 65535)
            .ok_or_else(|| format!("mihomo 配置里的 {key} 缺失或无效。"))?;
        inbound_ports.push(port);
    }
    Ok(Configs { mode: mode.into(), inbound_ports })
}

fn controller_url(address: &str) -> Result<reqwest::Url, String> {
    endpoint(address, "/configs")
}

fn endpoint(address: &str, path: &str) -> Result<reqwest::Url, String> {
    let address = address.trim().trim_end_matches('/');
    if address.is_empty() {
        return Err("controller 地址为空。".into());
    }
    let full = if address.contains("://") { address.to_string() } else { format!("http://{address}") };
    let url = reqwest::Url::parse(&format!("{full}{path}")).map_err(|_| "controller 地址格式无效。")?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("controller 地址格式无效。".into());
    }
    Ok(url)
}

pub(crate) enum Fetch<T> {
    Ok(T),
    Unauthorized,
    Failed(String),
}

/// 只读 GET 用户 mihomo 的控制接口；不走任何代理、不跟随跳转，超时、非 200、过大、非 JSON 都不是成功。
pub(crate) async fn get(controller: &str, secret: &str, path: &str, limit: usize) -> Fetch<Value> {
    let url = match endpoint(controller, path) {
        Ok(url) => url,
        Err(e) => return Fetch::Failed(e),
    };
    let client = match reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(client) => client,
        Err(_) => return Fetch::Failed("无法建立 controller 连接。".into()),
    };
    let mut request = client.get(url);
    if !secret.is_empty() {
        request = request.bearer_auth(secret);
    }
    let response = match request.send().await {
        Ok(r) => r,
        Err(e) if e.is_timeout() => return Fetch::Failed("读取 mihomo 控制接口超时。".into()),
        Err(_) => return Fetch::Failed("连不上 mihomo 控制接口。".into()),
    };
    match response.status().as_u16() {
        200 => {}
        401 | 403 => return Fetch::Unauthorized,
        code => return Fetch::Failed(format!("mihomo 控制接口返回 HTTP {code}。")),
    }
    let bytes = match response.bytes().await {
        Ok(b) if b.len() <= limit => b,
        Ok(_) => return Fetch::Failed("mihomo 控制接口响应过大。".into()),
        Err(_) => return Fetch::Failed("mihomo 控制接口响应读取失败。".into()),
    };
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(data) => Fetch::Ok(data),
        Err(_) => Fetch::Failed("mihomo 控制接口响应格式错误。".into()),
    }
}

/// 设置页校验 controller 地址用；与实际请求同一套解析。
pub fn controller_valid(address: &str) -> Result<(), String> {
    controller_url(address).map(drop)
}

/// 设置读不出来等情况下的「无法确认」，带原因。
pub fn unconfirmed(reason: String) -> Outcome {
    outcome(Verdict::Unknown, reason, "", "")
}

const BUILT_IN: [&str; 5] = ["DIRECT", "REJECT", "REJECT-DROP", "PASS", "COMPATIBLE"];

/// /proxies 里可指定的名字：代理组（带 all 列表）在前、节点在后，各自按名字排序；去掉内置出站与 GLOBAL。
pub fn node_names(data: &Value) -> Result<Vec<String>, String> {
    let proxies = data["proxies"].as_object().ok_or("mihomo 节点列表格式不对。")?;
    let (mut groups, mut nodes): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    for (name, item) in proxies {
        if BUILT_IN.contains(&name.as_str()) || name == "GLOBAL" {
            continue;
        }
        if item["all"].as_array().is_some() { groups.push(name.clone()) } else { nodes.push(name.clone()) }
    }
    groups.sort();
    nodes.sort();
    groups.extend(nodes);
    Ok(groups)
}

pub async fn nodes(controller: &str, secret: &str) -> Result<Vec<String>, String> {
    if controller.trim().is_empty() {
        return Err("还没设置 mihomo controller 地址。".into());
    }
    match get(controller, secret, "/proxies", 8 * 1024 * 1024).await {
        Fetch::Ok(data) => node_names(&data),
        Fetch::Unauthorized => Err("mihomo 拒绝了 secret，读不到节点。".into()),
        Fetch::Failed(e) => Err(e),
    }
}

async fn fetch_configs(controller: &str, secret: &str) -> Fetch<Configs> {
    match get(controller, secret, "/configs", 262_144).await {
        Fetch::Ok(data) => match parse_configs(&data) {
            Ok(configs) => Fetch::Ok(configs),
            Err(e) => Fetch::Failed(e),
        },
        Fetch::Unauthorized => Fetch::Unauthorized,
        Fetch::Failed(e) => Fetch::Failed(e),
    }
}

/// 各服务各自经专用端口给出的出口 IP，按出口结论裁决；只有全部成功、彼此一致且等于期望才通过。
#[cfg(test)]
pub fn decide(expected: IpAddr, observations: &[Result<Exit, String>], mode: &str) -> Outcome {
    decide_against(Some(expected), observations, mode)
}

/// 未设定期望出口时照常探测：一致也只给「无法确认」，并带回实际观测值供「记住这个出口」。
pub fn decide_against(expected: Option<IpAddr>, observations: &[Result<Exit, String>], mode: &str) -> Outcome {
    if observations.len() < 2 {
        return outcome(Verdict::Unknown, "出口查询服务少于 2 个，无法确认。", mode, "");
    }
    let mut ips = Vec::new();
    // 个别服务限流或连不上可以容忍；成功的至少 2 个且彼此一致才算数，任何不一致都拦住。
    let mut exits = Vec::new();
    let mut errors = Vec::new();
    for item in observations {
        match item {
            Ok(exit) => match exit.ip.parse::<IpAddr>() {
                Ok(ip) => {
                    ips.push(ip);
                    exits.push(exit);
                }
                Err(_) => errors.push("返回的 IP 无效".to_string()),
            },
            Err(e) => errors.push(e.clone()),
        }
    }
    if let Some(first) = ips.first() {
        if ips.iter().any(|ip| ip != first) {
            let seen: Vec<String> = ips.iter().map(IpAddr::to_string).collect();
            return outcome(Verdict::Block, format!("各查询服务看到的出口不一致（{}），已拦住。", seen.join("、")), mode, "");
        }
    }
    if ips.len() < 2 {
        return outcome(
            Verdict::Unknown,
            format!("经专用端口能给出结果的出口查询服务不足 2 个，无法确认：{}", errors.join("；")),
            mode,
            "",
        );
    }
    let first = ips[0];
    // 时区取第一个带时区的结果（Cloudflare trace 不带时区）。
    let exit = exits.iter().find(|e| !e.timezone.is_empty()).copied().unwrap_or(exits[0]);
    let Some(expected) = expected else {
        return observed(
            outcome(Verdict::Unknown, format!("专用线路出口为 {first}，还没设定期望出口，无法确认。"), mode, &first.to_string()),
            exit,
        );
    };
    if first != expected {
        return observed(
            outcome(
                Verdict::Block,
                format!("实际出口 {first} 与期望出口 {expected} 不符，已拦住。"),
                mode,
                &first.to_string(),
            ),
            exit,
        );
    }
    observed(outcome(Verdict::Pass, format!("专用线路出口为 {first}，与期望一致。"), mode, &first.to_string()), exit)
}

/// 互相印证的查询服务按域名计数：同一域名换路径、大小写或端口只算一个；非 http(s) 或无域名的地址不计。
pub fn distinct_services(services: &[String]) -> Vec<&str> {
    let mut hosts = Vec::new();
    let mut kept = Vec::new();
    for service in services {
        let service = service.trim();
        let Some(host) = reqwest::Url::parse(service).ok()
            .filter(|url| matches!(url.scheme(), "http" | "https"))
            .and_then(|url| url.host_str().map(str::to_ascii_lowercase)) else { continue };
        if !hosts.contains(&host) {
            hosts.push(host);
            kept.push(service);
        }
    }
    kept
}

pub async fn check(input: &Input) -> Outcome {
    if input.controller.trim().is_empty() {
        return outcome(Verdict::Unknown, "还没设置 mihomo controller 地址，无法确认。", "", "");
    }
    if input.port == 0 {
        return outcome(Verdict::Unknown, "还没设置 Claude 专用端口，无法确认。", "", "");
    }
    let expected = match input.expected_ip.trim() {
        "" => None,
        text => match text.parse::<IpAddr>() {
            Ok(ip) => Some(ip),
            Err(_) => return outcome(Verdict::Block, "期望出口 IP 格式无效。", "", ""),
        },
    };
    let services = distinct_services(&input.services);
    if services.len() < 2 {
        return outcome(Verdict::Unknown, "出口查询服务少于 2 个，无法确认。", "", "");
    }
    let configs = match fetch_configs(&input.controller, &input.secret).await {
        Fetch::Ok(configs) => configs,
        Fetch::Unauthorized => return outcome(Verdict::Block, "mihomo 拒绝了 secret，读不到配置，已拦住。", "", ""),
        Fetch::Failed(e) => return outcome(Verdict::Unknown, format!("{e}无法确认。"), "", ""),
    };
    if configs.inbound_ports.contains(&u64::from(input.port)) {
        return outcome(
            Verdict::Block,
            format!("端口 {} 是 mihomo 的通用入站端口，会跟着日常分流走，不是 Claude 专用线路，已拦住。", input.port),
            &configs.mode,
            "",
        );
    }
    let proxy = format!("http://127.0.0.1:{}", input.port);
    let mut observations = Vec::new();
    for service in services {
        observations.push(probe_urls(&proxy, &[service]).await);
    }
    decide_against(expected, &observations, &configs.mode)
}
