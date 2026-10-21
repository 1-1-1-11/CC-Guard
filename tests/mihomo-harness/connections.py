# [INPUT]: 本机 D:/clash_verge/verge-mihomo.exe（只执行，不读其目录）、curl.exe、cargo、src-tauri 中的 mihomo::live::live_connections 实测用例。
# [OUTPUT]: 每个场景一行 PASS/FAIL，末行 N/N PASS；有失败非零退出。
# [POS]: Claude 连接核对（漏网检查）的离线判卷台；只监听 127.0.0.1，不访问外网，不碰用户正在运行的 Clash，只清理自己起的子进程。冻结后不改。
import json
import os
import shutil
import socket
import socketserver
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request
from pathlib import Path

MIHOMO = Path(os.environ.get("MIHOMO_BIN", "D:/clash_verge/verge-mihomo.exe"))
VERSION = "v1.19.32"
ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / "src-tauri" / "Cargo.toml"
TEST_NAME = "mihomo::live::live_connections"
SECRET = "harness-secret"
NODE = "A"
HOSTS = ["anthropic.com", "claude.ai", "claude.com"]
SCENARIOS = [
    "判卷台自检",
    "Claude 域名经专用入口走 A→通过",
    "经普通入口按规则走 A→通过",
    "经普通入口走 B→拦住且列出域名",
    "名单进程访问其他域名走 B→拦住",
    "名单写 CURL.EXE→仍拦住",
    "无关域名、无关进程走 B→通过",
    "没有连接→通过",
    "off→仍按域名拦住且 process_matching=false",
    "secret 错→拦住",
    "mihomo 停→拦住",
]
CHILDREN = []


# ----- 慢回应假上游：HTTP 代理，隧道里收到请求后一直等到放行才回，保证查询时连接仍打开 -----
class Upstream(socketserver.ThreadingTCPServer):
    daemon_threads = True
    allow_reuse_address = False

    def __init__(self, port, ip):
        self.ip = ip
        self.release = threading.Event()
        super().__init__(("127.0.0.1", port), Handler)


def read_head(conn):
    data = b""
    while b"\r\n\r\n" not in data:
        chunk = conn.recv(4096)
        if not chunk:
            return None
        data += chunk
        if len(data) > 65536:
            return None
    return data.split(b"\r\n\r\n", 1)[0].decode("latin-1")


class Handler(socketserver.BaseRequestHandler):
    def handle(self):
        conn = self.request
        conn.settimeout(150)
        try:
            head = read_head(conn)
            if head is None:
                return
            if head.split(" ", 1)[0].upper() == "CONNECT":
                conn.sendall(b"HTTP/1.1 200 Connection established\r\n\r\n")
                head = read_head(conn)
                if head is None:
                    return
            if "x-harness-fast: 1" not in head.lower():
                self.server.release.wait(120)
            body = json.dumps({"ip": self.server.ip}).encode()
            conn.sendall(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: "
                + str(len(body)).encode()
                + b"\r\n\r\n"
                + body
            )
        except OSError:
            pass


def start_upstream(port, ip):
    server = Upstream(port, ip)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def stop_upstream(server):
    server.release.set()
    server.shutdown()
    server.server_close()


# ----- 工具 -----
def free_ports(n):
    socks = []
    for _ in range(n):
        s = socket.socket()
        s.bind(("127.0.0.1", 0))
        socks.append(s)
    ports = [s.getsockname()[1] for s in socks]
    for s in socks:
        s.close()
    return ports


def fast_ip(port, host):
    """经 127.0.0.1:port 打隧道到 host，带快速标记立即取回假上游的 IP；失败返回 None。"""
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=10) as s:
            s.sendall(f"CONNECT {host}:80 HTTP/1.1\r\nHost: {host}:80\r\n\r\n".encode())
            head = read_head(s)
            if head is None or " 200 " not in head.split("\r\n", 1)[0] + " ":
                return None
            s.sendall(f"GET / HTTP/1.1\r\nHost: {host}\r\nX-Harness-Fast: 1\r\nConnection: close\r\n\r\n".encode())
            data = b""
            while True:
                chunk = s.recv(4096)
                if not chunk:
                    break
                data += chunk
        return json.loads(data.partition(b"\r\n\r\n")[2].decode())["ip"]
    except (OSError, ValueError, KeyError):
        return None


def hold(port, host):
    """本进程（python）经 127.0.0.1:port 打隧道到 host 并发出请求，连接保持到放行。"""
    s = socket.create_connection(("127.0.0.1", port), timeout=10)
    s.sendall(f"CONNECT {host}:80 HTTP/1.1\r\nHost: {host}:80\r\n\r\n".encode())
    head = read_head(s)
    if head is None or " 200 " not in head.split("\r\n", 1)[0] + " ":
        s.close()
        raise OSError("隧道未建立")
    s.sendall(f"GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n".encode())
    return s


def curl_hold(curl, port, host):
    """curl.exe 经 127.0.0.1:port 打隧道到 host，等待假上游放行。"""
    env = {k: v for k, v in os.environ.items() if k.lower() not in ("http_proxy", "https_proxy", "all_proxy", "no_proxy")}
    proc = subprocess.Popen(
        [curl, "-s", "-p", "-x", f"http://127.0.0.1:{port}", "--max-time", "120", f"http://{host}/"],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    CHILDREN.append(proc)
    return proc


def controller(port, method, path, body=None, secret=SECRET):
    req = urllib.request.Request(f"http://127.0.0.1:{port}{path}", method=method)
    req.add_header("Authorization", f"Bearer {secret}")
    if body is not None:
        req.add_header("Content-Type", "application/json")
        req.data = json.dumps(body).encode()
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        with opener.open(req, timeout=5) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        return e.code, b""
    except OSError:
        return None, b""


def live_connections(port):
    status, body = controller(port, "GET", "/connections")
    if status != 200:
        return None
    try:
        return json.loads(body.decode()).get("connections") or []
    except ValueError:
        return None


def wait_connections(port, want, timeout=20):
    """等 mihomo 里出现恰好 want 条连接（want 为 0 即等清空）；返回连接列表或 None。"""
    deadline = time.time() + timeout
    while time.time() < deadline:
        conns = live_connections(port)
        if conns is not None and len(conns) == want:
            return conns
        time.sleep(0.2)
    return None


def kill_tree(proc):
    if proc.poll() is None:
        subprocess.run(["taskkill", "/T", "/F", "/PID", str(proc.pid)], capture_output=True)
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()


def cargo_env(extra):
    env = dict(os.environ)
    target = env.get("CARGO_TARGET_DIR") or str(ROOT.parent / "ccbaga-build-target")
    forbidden = (ROOT / "src-tauri" / "target").resolve()
    resolved = Path(target).resolve()
    if resolved == forbidden or forbidden in resolved.parents:
        raise SystemExit("CARGO_TARGET_DIR 不许指向 src-tauri/target")
    env["CARGO_TARGET_DIR"] = target
    for key in ("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"):
        env.pop(key, None)
    env["NO_PROXY"] = "*"
    env.update(extra)
    return env


def run_cargo(args, extra, timeout):
    cargo = shutil.which("cargo")
    if not cargo:
        return None, "找不到 cargo"
    proc = subprocess.Popen(
        [cargo, *args],
        cwd=ROOT,
        env=cargo_env(extra),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    CHILDREN.append(proc)
    try:
        out, err = proc.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        kill_tree(proc)
        return None, "cargo 超时"
    return proc.returncode, out.decode("utf-8", "replace") + "\n" + err.decode("utf-8", "replace")


def check(ctl, secret, processes):
    payload = json.dumps(
        {"controller": ctl, "secret": secret, "node": NODE, "claude_hosts": HOSTS, "claude_processes": processes}
    )
    code, out = run_cargo(
        ["test", "--manifest-path", str(MANIFEST), TEST_NAME, "--", "--ignored", "--exact", "--nocapture", "--test-threads=1"],
        {"MIHOMO_LIVE_INPUT": payload},
        900,
    )
    if code != 0 or "test result: ok. 1 passed" not in out:
        return None, f"实测用例未成功运行（exit={code}）：{out[-600:]}"
    for line in out.splitlines():
        if "MIHOMO_CONNECTIONS " in line:
            try:
                return json.loads(line.split("MIHOMO_CONNECTIONS ", 1)[1]), ""
            except ValueError:
                return None, "结果不是 JSON"
    return None, "实测用例没有输出结果"


# ----- 判定 -----
def reasoned(r):
    return isinstance(r.get("reason"), str) and bool(r["reason"]) and isinstance(r.get("checked_at"), int) and r["checked_at"] > 0


def passed(r, claude, matching=True):
    return (
        r["verdict"] == "pass"
        and r["leaks"] == []
        and r["claude"] == claude
        and r["total"] >= 1
        and r["process_matching"] is matching
        and reasoned(r)
    )


def leaked(r, host, process=None, matching=True, in_reason=False):
    hits = [
        leak
        for leak in r["leaks"]
        if leak["host"] == host and leak["node"] == "B" and (process is None or leak["process"].lower() == process)
    ]
    return (
        r["verdict"] == "block"
        and bool(hits)
        and r["process_matching"] is matching
        and (not in_reason or host in r["reason"])
        and reasoned(r)
    )


def blocked(r):
    return r["verdict"] in ("block", "unknown") and bool(r.get("reason"))


def nothing(r):
    return r["verdict"] == "pass" and r["total"] == 0 and r["leaks"] == [] and "此刻没有连接" in r["reason"]


def write_config(tmp, name, ports, mode):
    p_a, p_b, p_mixed, p_ctl, p_claude = ports
    path = tmp / name
    path.write_text(
        "\n".join(
            [
                f"mixed-port: {p_mixed}",
                "allow-lan: false",
                "bind-address: 127.0.0.1",
                f"external-controller: 127.0.0.1:{p_ctl}",
                f"secret: {SECRET}",
                "mode: rule",
                "log-level: warning",
                "ipv6: false",
                "geo-auto-update: false",
                f"find-process-mode: {mode}",
                "profile: {store-selected: false, store-fake-ip: false}",
                "dns: {enable: false}",
                "tun: {enable: false}",
                "sniffer: {enable: false}",
                "proxies:",
                f"  - {{name: A, type: http, server: 127.0.0.1, port: {p_a}}}",
                f"  - {{name: B, type: http, server: 127.0.0.1, port: {p_b}}}",
                "proxy-groups:",
                "  - {name: daily, type: select, proxies: [B, A]}",
                "rules:",
                "  - DOMAIN-SUFFIX,claude.ai,A",
                "  - MATCH,daily",
                "listeners:",
                f"  - {{name: claude, type: mixed, listen: 127.0.0.1, port: {p_claude}, proxy: A}}",
                "",
            ]
        ),
        encoding="utf-8",
    )
    return path


def start_mihomo(tmp, config, p_ctl, tag):
    log = open(tmp / f"mihomo-{tag}.log", "wb")
    proc = subprocess.Popen([str(MIHOMO), "-d", str(tmp), "-f", str(config)], cwd=tmp, stdout=log, stderr=subprocess.STDOUT)
    CHILDREN.append(proc)
    for _ in range(100):
        if controller(p_ctl, "GET", "/version")[0] == 200:
            break
        time.sleep(0.2)
    return proc


# ----- 主流程 -----
def main():
    sys.stdout.reconfigure(encoding="utf-8")
    tmp = Path(tempfile.mkdtemp(prefix="mihomo-connections-"))
    results = {}
    notes = {}
    upstreams = []
    curl = shutil.which("curl.exe")
    try:
        ports = free_ports(5)
        p_a, p_b, p_mixed, p_ctl, p_claude = ports
        upstream_a = start_upstream(p_a, "203.0.113.10")
        upstream_b = start_upstream(p_b, "198.51.100.20")
        upstreams = [upstream_a, upstream_b]
        always = write_config(tmp, "always.yaml", ports, "always")
        off = write_config(tmp, "off.yaml", ports, "off")
        ctl = f"127.0.0.1:{p_ctl}"
        mihomo = None

        def settle():
            """放行全部挂起请求，等 mihomo 里的连接清空，再为下一场景换新闸门。"""
            for server in upstreams:
                server.release.set()
            ok = wait_connections(p_ctl, 0) is not None
            for server in upstreams:
                server.release = threading.Event()
            return ok

        # ----- 自检 -----
        problems = []
        if not MIHOMO.is_file():
            problems.append("找不到 verge-mihomo.exe")
        elif not curl:
            problems.append("找不到 curl.exe")
        else:
            v = subprocess.run([str(MIHOMO), "-v"], capture_output=True, timeout=30)
            if VERSION not in v.stdout.decode("utf-8", "replace").split():
                problems.append(f"内核版本不是 {VERSION}")
        if not problems:
            mihomo = start_mihomo(tmp, always, p_ctl, "always")
            status, body = controller(p_ctl, "GET", "/version")
            if status != 200 or VERSION not in body.decode("utf-8", "replace"):
                problems.append("独立 mihomo 未就绪或版本不符")
            elif controller(p_ctl, "GET", "/version", secret="wrong")[0] != 401:
                problems.append("错误 secret 未被拒绝")
            elif fast_ip(p_claude, "claude.com") != "203.0.113.10":
                problems.append("专用入口没有走 A")
            elif fast_ip(p_mixed, "claude.ai") != "203.0.113.10":
                problems.append("claude.ai 规则没有走 A")
            elif fast_ip(p_mixed, "api.anthropic.com") != "198.51.100.20":
                problems.append("普通入口默认没有走 B")
            elif not settle():
                problems.append("快速请求后连接没有清空")
        if not problems:
            proc = curl_hold(curl, p_mixed, "other.test")
            conns = wait_connections(p_ctl, 1)
            if not conns:
                problems.append("curl 的连接没有出现在 /connections")
            else:
                meta = conns[0].get("metadata", {})
                if meta.get("process", "").lower() != "curl.exe" or conns[0].get("chains", [None])[0] != "B":
                    problems.append(f"always 模式下没认出 curl.exe 或没走 B：{json.dumps(conns[0], ensure_ascii=False)[:300]}")
            if not settle():
                problems.append("curl 连接没有清空")
            try:
                proc.wait(timeout=20)
            except subprocess.TimeoutExpired:
                kill_tree(proc)
        if not problems:
            code, out = run_cargo(["test", "--manifest-path", str(MANIFEST), "--no-run"], {}, 1800)
            if code != 0:
                problems.append(f"cargo test 编译失败：{(out or '')[-600:]}")
        results[SCENARIOS[0]] = not problems
        notes[SCENARIOS[0]] = "；".join(problems)

        if results[SCENARIOS[0]]:

            def scenario(name, ok_fn, opens, processes=("claude.exe",), secret=SECRET):
                """opens：[(方式, 端口, 域名)]；先确认连接都在，再调实测用例，最后放行清场。"""
                sockets, procs = [], []
                try:
                    for how, port, host in opens:
                        if how == "curl":
                            procs.append(curl_hold(curl, port, host))
                        else:
                            sockets.append(hold(port, host))
                    if wait_connections(p_ctl, len(opens)) is None:
                        results[name], notes[name] = False, "连接没有按预期出现在 /connections"
                        return
                    result, err = check(ctl, secret, list(processes))
                    if result is None:
                        results[name], notes[name] = False, err
                        return
                    results[name] = ok_fn(result)
                    notes[name] = json.dumps(result, ensure_ascii=False)
                except OSError as e:
                    results[name], notes[name] = False, f"建立连接失败：{e}"
                finally:
                    for s in sockets:
                        s.close()
                    if not settle():
                        results[name] = False
                        notes[name] = notes.get(name, "") + "；场景后连接没有清空"
                    for proc in procs:
                        try:
                            proc.wait(timeout=20)
                        except subprocess.TimeoutExpired:
                            kill_tree(proc)

            scenario(SCENARIOS[1], lambda r: passed(r, 1), [("py", p_claude, "api.anthropic.com")])
            scenario(SCENARIOS[2], lambda r: passed(r, 1), [("py", p_mixed, "claude.ai")])
            scenario(
                SCENARIOS[3],
                lambda r: leaked(r, "api.anthropic.com", in_reason=True) and r["claude"] == 2,
                [("py", p_mixed, "api.anthropic.com"), ("py", p_mixed, "claude.ai")],
            )
            scenario(SCENARIOS[4], lambda r: leaked(r, "other.test", process="curl.exe"), [("curl", p_mixed, "other.test")], processes=["curl.exe"])
            scenario(SCENARIOS[5], lambda r: leaked(r, "other.test", process="curl.exe"), [("curl", p_mixed, "other.test")], processes=["CURL.EXE"])
            scenario(
                SCENARIOS[6],
                lambda r: passed(r, 0),
                [("py", p_mixed, "other.test"), ("curl", p_mixed, "example.test")],
            )
            if wait_connections(p_ctl, 0) is not None:
                result, err = check(ctl, SECRET, ["claude.exe"])
                results[SCENARIOS[7]] = result is not None and nothing(result)
                notes[SCENARIOS[7]] = json.dumps(result, ensure_ascii=False) if result else err
            else:
                results[SCENARIOS[7]], notes[SCENARIOS[7]] = False, "连接没有清空"

            # 换成 find-process-mode: off 的同一份配置
            kill_tree(mihomo)
            if controller(p_ctl, "GET", "/version")[0] is not None:
                results[SCENARIOS[8]], notes[SCENARIOS[8]] = False, "旧 mihomo 未停止"
            else:
                mihomo = start_mihomo(tmp, off, p_ctl, "off")
                status, body = controller(p_ctl, "GET", "/configs")
                mode = json.loads(body.decode()).get("find-process-mode") if status == 200 else None
                if mode != "off":
                    results[SCENARIOS[8]], notes[SCENARIOS[8]] = False, f"off 配置未生效：{mode}"
                else:
                    scenario(
                        SCENARIOS[8],
                        lambda r: leaked(r, "api.anthropic.com", matching=False),
                        [("py", p_mixed, "api.anthropic.com")],
                    )
            scenario(SCENARIOS[9], blocked, [("py", p_mixed, "api.anthropic.com")], secret="wrong-secret")

            kill_tree(mihomo)
            if controller(p_ctl, "GET", "/version")[0] is None:
                result, err = check(ctl, SECRET, ["claude.exe"])
                results[SCENARIOS[10]] = result is not None and blocked(result)
                notes[SCENARIOS[10]] = json.dumps(result, ensure_ascii=False) if result else err
            else:
                results[SCENARIOS[10]], notes[SCENARIOS[10]] = False, "mihomo 未停止"
    finally:
        for proc in CHILDREN:
            kill_tree(proc)
        for server in upstreams:
            stop_upstream(server)
        shutil.rmtree(tmp, ignore_errors=True)

    ok_count = 0
    for name in SCENARIOS:
        ok = results.get(name, False) and results.get(SCENARIOS[0], False)
        ok_count += ok
        note = notes.get(name, "自检未通过，不判" if not results.get(SCENARIOS[0]) else "未运行")
        print(f"{'PASS' if ok else 'FAIL'}  {name}  {note}")
    print(f"{ok_count}/{len(SCENARIOS)} PASS")
    return 0 if ok_count == len(SCENARIOS) else 1


if __name__ == "__main__":
    sys.exit(main())
