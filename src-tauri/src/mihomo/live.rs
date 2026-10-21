// [INPUT]: tests/mihomo-harness/{run,connections}.py 经 MIHOMO_LIVE_INPUT 传入的真实 mihomo 与假上游参数。
// [OUTPUT]: 一行 MIHOMO_RESULT / MIHOMO_CONNECTIONS JSON；判定由判卷台做，本用例不自带断言。
// [POS]: 需判卷台调起的实测用例，单独 cargo test 时忽略。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::{check, Input};
use serde_json::Value;

#[test]
#[ignore = "需 tests/mihomo-harness/run.py 起真实 mihomo 与假上游后调起"]
fn live_check() {
    let raw = std::env::var("MIHOMO_LIVE_INPUT").expect("缺少 MIHOMO_LIVE_INPUT");
    let data: Value = serde_json::from_str(&raw).expect("MIHOMO_LIVE_INPUT 不是 JSON");
    let input = Input {
        controller: data["controller"].as_str().expect("controller").into(),
        secret: data["secret"].as_str().expect("secret").into(),
        port: u16::try_from(data["port"].as_u64().expect("port")).expect("port"),
        expected_ip: data["expected_ip"].as_str().expect("expected_ip").into(),
        services: data["services"]
            .as_array()
            .expect("services")
            .iter()
            .map(|s| s.as_str().expect("service").to_string())
            .collect(),
    };
    let result = tauri::async_runtime::block_on(check(&input));
    println!("MIHOMO_RESULT {}", serde_json::to_string(&result).unwrap());
}

#[test]
#[ignore = "需 tests/mihomo-harness/connections.py 起真实 mihomo 与慢回应假上游后调起"]
fn live_connections() {
    let raw = std::env::var("MIHOMO_LIVE_INPUT").expect("缺少 MIHOMO_LIVE_INPUT");
    let data: Value = serde_json::from_str(&raw).expect("MIHOMO_LIVE_INPUT 不是 JSON");
    let list = |key: &str| -> Vec<String> {
        data[key].as_array().expect(key).iter().map(|s| s.as_str().expect(key).to_string()).collect()
    };
    let input = super::connections::Input {
        controller: data["controller"].as_str().expect("controller").into(),
        secret: data["secret"].as_str().expect("secret").into(),
        node: data["node"].as_str().expect("node").into(),
        claude_hosts: list("claude_hosts"),
        claude_processes: list("claude_processes"),
    };
    let result = tauri::async_runtime::block_on(super::connections::check(&input));
    println!("MIHOMO_CONNECTIONS {}", serde_json::to_string(&result).unwrap());
}
