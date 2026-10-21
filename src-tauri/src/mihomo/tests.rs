// [INPUT]: 手写 /configs 样本、出口观测与本机必然连不上的地址。
// [OUTPUT]: 解析、裁决与输入校验的回归；读不到、格式不对、不一致都不是通过。
// [POS]: 无网络、无 mihomo 的纯逻辑单测；真实链路由判卷台覆盖。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::*;
use serde_json::json;

fn exit(ip: &str) -> Result<Exit, String> {
    Ok(Exit { ip: ip.into(), country: "US".into(), ..Default::default() })
}
fn configs() -> Value {
    json!({"mode": "rule", "port": 0, "socks-port": 0, "mixed-port": 7897, "redir-port": 0, "tproxy-port": 0})
}
fn input(services: &[&str]) -> Input {
    Input {
        // 端口 1 在本机没有服务，连接会被立即拒绝
        controller: "127.0.0.1:1".into(),
        secret: "s".into(),
        port: 7899,
        expected_ip: "203.0.113.10".into(),
        services: services.iter().map(|s| s.to_string()).collect(),
    }
}
fn run(input: &Input) -> Outcome {
    tauri::async_runtime::block_on(check(input))
}

#[test]
fn parses_mode_and_every_inbound_port() {
    let parsed = parse_configs(&configs()).unwrap();
    assert_eq!(parsed, Configs { mode: "rule".into(), inbound_ports: vec![0, 0, 7897, 0, 0] });
}

#[test]
fn rejects_configs_missing_mode_or_any_port() {
    for key in ["mode", "port", "socks-port", "mixed-port", "redir-port", "tproxy-port"] {
        let mut data = configs();
        data.as_object_mut().unwrap().remove(key);
        assert!(parse_configs(&data).is_err(), "{key}");
    }
    let mut data = configs();
    data["mixed-port"] = json!("7897");
    assert!(parse_configs(&data).is_err());
    data["mixed-port"] = json!(70000);
    assert!(parse_configs(&data).is_err());
    assert!(parse_configs(&json!([])).is_err());
}

#[test]
fn passes_only_when_all_services_agree_on_expected_ip() {
    let expected: IpAddr = "203.0.113.10".parse().unwrap();
    let ok = decide(expected, &[exit("203.0.113.10"), exit("203.0.113.10")], "rule");
    assert_eq!(ok.verdict, Verdict::Pass);
    assert_eq!((ok.ip.as_str(), ok.mode.as_str()), ("203.0.113.10", "rule"));

    let split = decide(expected, &[exit("203.0.113.10"), exit("203.0.113.99")], "rule");
    assert_eq!(split.verdict, Verdict::Block);
    let wrong = decide(expected, &[exit("198.51.100.20"), exit("198.51.100.20")], "rule");
    assert_eq!(wrong.verdict, Verdict::Block);
    let failed = decide(expected, &[exit("203.0.113.10"), Err("HTTP 502".into())], "rule");
    assert_eq!(failed.verdict, Verdict::Unknown);
    let single = decide(expected, &[exit("203.0.113.10")], "rule");
    assert_eq!(single.verdict, Verdict::Unknown);
    let garbage = decide(expected, &[exit("203.0.113.10"), exit("not-an-ip")], "rule");
    assert_eq!(garbage.verdict, Verdict::Unknown);
}

#[test]
fn every_outcome_has_a_reason() {
    let expected: IpAddr = "203.0.113.10".parse().unwrap();
    for obs in [vec![exit("203.0.113.10"), exit("203.0.113.10")], vec![exit("1.1.1.1"), exit("1.1.1.1")], vec![]] {
        assert!(!decide(expected, &obs, "rule").reason.is_empty());
    }
}

#[test]
fn bad_inputs_never_pass() {
    let mut bad_ip = input(&["http://a.test/", "http://b.test/"]);
    bad_ip.expected_ip = "not-an-ip".into();
    assert_eq!(run(&bad_ip).verdict, Verdict::Block);

    // 未设置完整按契约是「无法确认」，原因说明缺什么
    let mut zero = input(&["http://a.test/", "http://b.test/"]);
    zero.port = 0;
    let unset = run(&zero);
    assert_eq!(unset.verdict, Verdict::Unknown);
    assert!(unset.reason.contains("专用端口"));
    let mut blank = input(&["http://a.test/", "http://b.test/"]);
    blank.controller = " ".into();
    let unset = run(&blank);
    assert_eq!(unset.verdict, Verdict::Unknown);
    assert!(unset.reason.contains("controller"));

    // 去重后不足 2 个服务，在连 controller 之前就判无法确认
    assert_eq!(run(&input(&["http://a.test/", "http://a.test/again"])).verdict, Verdict::Unknown);

    let mut bad_controller = input(&["http://a.test/", "http://b.test/"]);
    bad_controller.controller = "ftp://127.0.0.1:1".into();
    assert_eq!(run(&bad_controller).verdict, Verdict::Unknown);
}

#[test]
fn one_host_counts_as_one_service() {
    let list = |items: &[&str]| -> Vec<String> {
        let owned: Vec<String> = items.iter().map(|s| s.to_string()).collect();
        distinct_services(&owned).into_iter().map(str::to_owned).collect()
    };
    assert_eq!(list(&["http://a.test/", " http://a.test/ "]).len(), 1);
    assert_eq!(list(&["http://a.test/", "http://A.test/again"]).len(), 1);
    assert_eq!(list(&["http://a.test/", "http://a.test:8080/"]).len(), 1);
    assert_eq!(list(&["http://a.test/", "not a url", "ftp://b.test/", ""]).len(), 1);
    assert_eq!(list(&["http://a.test/", "https://b.test/x"]), vec!["http://a.test/", "https://b.test/x"]);
}

#[test]
fn unreachable_controller_is_unknown() {
    let result = run(&input(&["http://a.test/", "http://b.test/"]));
    assert_eq!(result.verdict, Verdict::Unknown);
    assert!(!result.reason.is_empty());
}

#[test]
fn controller_address_accepts_bare_host_and_url() {
    assert_eq!(controller_url("127.0.0.1:9090").unwrap().as_str(), "http://127.0.0.1:9090/configs");
    assert_eq!(controller_url("http://127.0.0.1:9090/").unwrap().as_str(), "http://127.0.0.1:9090/configs");
    assert!(controller_url("").is_err());
}

#[test]
fn unset_expected_ip_reports_observed_exit_but_never_passes() {
    let seen = |ip: &str| Ok(Exit { ip: ip.into(), country: "JP".into(), timezone: "Asia/Tokyo".into(), ..Default::default() });
    let result = decide_against(None, &[seen("203.0.113.10"), seen("203.0.113.10")], "rule");
    assert_eq!(result.verdict, Verdict::Unknown);
    assert_eq!((result.ip.as_str(), result.country.as_str(), result.timezone.as_str()), ("203.0.113.10", "JP", "Asia/Tokyo"));
    let split = decide_against(None, &[seen("203.0.113.10"), seen("203.0.113.99")], "rule");
    assert_eq!(split.verdict, Verdict::Block);
    let ok = decide("203.0.113.10".parse().unwrap(), &[seen("203.0.113.10"), seen("203.0.113.10")], "rule");
    assert_eq!((ok.verdict, ok.timezone.as_str()), (Verdict::Pass, "Asia/Tokyo"));
    assert!(ok.checked_at > 0);
}

mod connections {
    use super::super::connections::*;
    use super::super::Verdict;
    use serde_json::{json, Value};

    fn input(processes: &[&str]) -> Input {
        Input {
            controller: "127.0.0.1:1".into(),
            secret: "s".into(),
            node: "A".into(),
            claude_hosts: vec!["anthropic.com".into(), "claude.ai".into(), "claude.com".into()],
            claude_processes: processes.iter().map(|s| s.to_string()).collect(),
        }
    }
    fn conn(host: &str, process: &str, chains: Value) -> Value {
        json!({"metadata": {"host": host, "process": process, "inboundName": "DEFAULT-MIXED"}, "chains": chains})
    }

    #[test]
    fn suffixes_match_whole_labels_only() {
        let hosts = input(&[]).claude_hosts;
        for yes in ["claude.ai", "API.Anthropic.com", "statsig.anthropic.com.", "claude.com"] {
            assert!(host_matches(yes, &hosts), "{yes}");
        }
        for no in ["notclaude.ai", "anthropic.com.evil.test", "claude.aix", ""] {
            assert!(!host_matches(no, &hosts), "{no}");
        }
    }

    #[test]
    fn process_names_ignore_case_but_never_match_partially() {
        let names = vec!["claude.exe".to_string()];
        assert!(process_matches("CLAUDE.EXE", &names));
        assert!(process_matches(r"C:\Program Files\Claude\claude.exe", &names));
        for no in ["claude.exe.bak", "myclaude.exe", "claude", ""] {
            assert!(!process_matches(no, &names), "{no}");
        }
    }

    #[test]
    fn claude_traffic_off_the_node_is_a_leak() {
        let data = json!({"connections": [
            conn("api.anthropic.com", "", json!(["B", "daily"])),
            conn("claude.ai", "", json!(["A"])),
            conn("other.test", "Claude.exe", json!(["B"])),
            conn("other.test", "chrome.exe", json!(["B"])),
            conn("claude.com", "", json!([])),
        ]});
        let result = judge(&data, &input(&["claude.exe"]), true);
        assert_eq!(result.verdict, Verdict::Block);
        assert_eq!((result.total, result.claude, result.leaks.len()), (5, 4, 3));
        assert_eq!(result.leaks[0].node, "B");
        assert_eq!(result.leaks[1].process, "Claude.exe");
        assert_eq!(result.leaks[2].node, "");
        assert!(result.reason.contains("api.anthropic.com") && result.reason.contains("claude.com"));
    }

    #[test]
    fn a_group_counts_when_the_connection_passes_through_it() {
        // 真实配置：Claude → 固定出口组 → fallback 组 → 多个同 IP 静态出口，fallback 切换不算漏网。
        let mut group = input(&[]);
        group.node = "Claude-专线".into();
        let data = json!({"connections": [
            conn("api.anthropic.com", "", json!(["专线·节点1", "专线-自动切换", "Claude-专线"])),
            conn("claude.ai", "", json!(["专线·节点2", "专线-自动切换", "Claude-专线"])),
        ]});
        let ok = judge(&data, &group, true);
        assert_eq!((ok.verdict, ok.leaks.len()), (Verdict::Pass, 0));

        let data = json!({"connections": [conn("claude.ai", "", json!(["日常·节点1", "日常-自动切换"]))]});
        let leak = judge(&data, &group, true);
        assert_eq!(leak.verdict, Verdict::Block);
        assert_eq!(leak.leaks[0].node, "日常·节点1");
    }

    #[test]
    fn clean_or_empty_snapshots_pass_with_scope_stated() {
        let none = judge(&json!({"connections": null}), &input(&[]), false);
        assert_eq!((none.verdict, none.total), (Verdict::Pass, 0));
        assert!(none.reason.contains("此刻没有连接") && none.reason.contains("只按域名"));
        let fine = judge(&json!({"connections": [conn("claude.ai", "", json!(["A"]))]}), &input(&[]), true);
        assert_eq!((fine.verdict, fine.claude, fine.process_matching), (Verdict::Pass, 1, true));
    }

    #[test]
    fn malformed_snapshots_are_unknown() {
        for data in [json!({"connections": {}}), json!({"connections": [1]}), json!({"connections": [{"chains": ["A"]}]})] {
            assert_eq!(judge(&data, &input(&[]), true).verdict, Verdict::Unknown, "{data}");
        }
    }

    #[test]
    fn unset_or_unreachable_never_passes() {
        let run = |input: &Input| tauri::async_runtime::block_on(check(input));
        let mut blank = input(&[]);
        blank.node = String::new();
        assert_eq!(run(&blank).verdict, Verdict::Unknown);
        let mut no_hosts = input(&[]);
        no_hosts.claude_hosts = vec![" ".into()];
        assert_eq!(run(&no_hosts).verdict, Verdict::Unknown);
        assert_eq!(run(&input(&[])).verdict, Verdict::Unknown);
    }
}

#[test]
fn node_list_puts_groups_first_and_skips_built_ins() {
    let data = json!({"proxies": {
        "Tokyo-1": {"type": "Vmess"}, "A": {"type": "Http"}, "daily": {"type": "Selector", "all": ["A"]},
        "auto": {"type": "Fallback", "all": ["A"]},
        "DIRECT": {"type": "Direct"}, "REJECT": {"type": "Reject"}, "GLOBAL": {"type": "Selector", "all": []}
    }});
    let names: Vec<String> = ["auto", "daily", "A", "Tokyo-1"].iter().map(|s| s.to_string()).collect();
    assert_eq!(node_names(&data).unwrap(), names);
    assert!(node_names(&json!({"proxies": []})).is_err());
}

#[test]
fn ifconfig_and_cloudflare_trace_bodies_parse() {
    use crate::browser::network::parse_body;
    let ifconfig = br#"{"ip":"203.0.113.10","country_iso":"JP","time_zone":"Asia/Tokyo","asn_org":"x"}"#;
    let exit = parse_body(ifconfig).unwrap();
    assert_eq!((exit.ip.as_str(), exit.country.as_str(), exit.timezone.as_str()), ("203.0.113.10", "JP", "Asia/Tokyo"));
    let trace = b"fl=1\nh=www.cloudflare.com\nip=203.0.113.10\nloc=JP\ncolo=NRT\n";
    let exit = parse_body(trace).unwrap();
    assert_eq!((exit.ip.as_str(), exit.country.as_str(), exit.timezone.as_str()), ("203.0.113.10", "JP", ""));
    for bad in [&b"ip=203.0.113.10\n"[..], b"loc=JP\n", b"<html>429</html>", b"ip=not-ip\nloc=JP\n"] {
        assert!(parse_body(bad).is_err());
    }
}

#[test]
fn one_failing_service_is_tolerated_but_disagreement_still_blocks() {
    let expected: IpAddr = "203.0.113.10".parse().unwrap();
    let tz = |ip: &str, zone: &str| Ok(Exit { ip: ip.into(), country: "JP".into(), timezone: zone.into(), ..Default::default() });
    let ok = decide(expected, &[tz("203.0.113.10", ""), Err("HTTP 429".into()), tz("203.0.113.10", "Asia/Tokyo")], "rule");
    assert_eq!((ok.verdict, ok.timezone.as_str()), (Verdict::Pass, "Asia/Tokyo"));
    let split = decide(expected, &[tz("203.0.113.10", ""), Err("HTTP 429".into()), tz("198.51.100.9", "")], "rule");
    assert_eq!(split.verdict, Verdict::Block);
    let thin = decide(expected, &[tz("203.0.113.10", ""), Err("HTTP 429".into()), Err("timeout".into())], "rule");
    assert_eq!(thin.verdict, Verdict::Unknown);
    assert!(thin.reason.contains("429"));
}
