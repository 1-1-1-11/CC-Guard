# [INPUT]: 本机 D:/clash_verge/verge-mihomo.exe（只执行，不读其目录）、cargo、src-tauri 中的 mihomo::live::live_check 实测用例。
# [OUTPUT]: 每个场景一行 PASS/FAIL，末行 N/N PASS；有失败非零退出。
# [POS]: Claude 专用线路检查的离线判卷台；只监听 127.0.0.1，不访问外网，不碰用户正在运行的 Clash。冻结后不改。
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
TEST_NAME = "mihomo::live::live_check"
IP_A = "203.0.113.10"
IP_A_ODD = "203.0.113.99"
IP_B = "198.51.100.20"
SERVICES = ["http://svc1.test/", "http://svc2.test/"]
SERVICES_ODD = ["http://svc1.test/", "http://odd.test/"]
SECRET = "harness-secret"
SCENARIOS = [
    "判卷台自检",
    "正常→通过且 IP=A",
    "切日常组→仍通过",
    "A 挂→拦住",
    "mihomo 停→拦住",
    "secret 错→拦住",
    "期望 IP 不符→拦住",
    "查询服务不一致→拦住",
    "端口不存在→拦住",
    "端口填成 mixed-port→拦住",
]
CHILDREN = []


# ----- 假上游：HTTP 代理，收到 CONNECT 后自己在隧道里回出口 JSON -----
class Upstream(socketserver.ThreadingTCPServer):
    daemon_threads = True
    allow_reuse_address = False

    def __init__(self, port, ips):
        self.ips = ips
        self.hits = 0
        self.lock = threading.Lock()
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


def host_of(head):
    lines = head.split("\r\n")
    parts = lines[0].split(" ")
    if len(parts) >= 2 and parts[1].startswith("http://"):
        return parts[1][7:].split("/", 1)[0].split(":")[0].lower()
    for line in lines[1:]:
        if line.lower().startswith("host:"):
            return line[5:].strip().split(":")[0].lower()
    return ""


class Handler(socketserver.BaseRequestHandler):
    def handle(self):
        conn = self.request
        conn.settimeout(10)
        try:
            head = read_head(conn)
            if head is None:
                return
            if head.split(" ", 1)[0].upper() == "CONNECT":
                conn.sendall(b"HTTP/1.1 200 Connection established\r\n\r\n")
                head = read_head(conn)
                if head is None:
                    return
            host = host_of(head)
            ip = self.server.ips.get(host, self.server.ips["*"])
            with self.server.lock:
                self.server.hits += 1
            body = json.dumps({"ip": ip, "country_code": "US", "timezone": "America/Los_Angeles", "org": "Harness"}).encode()
            conn.sendall(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: "
                + str(len(body)).encode()
                + b"\r\n\r\n"
                + body
            )
        except OSError:
            pass


def start_upstream(port, ips):
    server = Upstream(port, ips)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def stop_upstream(server):
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


def ask(port, url):
    """经 127.0.0.1:port（当 HTTP 代理用）请求 url，返回出口 IP；失败返回 None。"""
    host = url[7:].split("/", 1)[0]
    try:
        with socket.create_connection(("127.0.0.1", port), timeout=10) as s:
            s.sendall(f"GET {url} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n".encode())
            data = b""
            while True:
                chunk = s.recv(4096)
                if not chunk:
                    break
                data += chunk
        head, _, body = data.partition(b"\r\n\r\n")
        if b" 200 " not in head.split(b"\r\n", 1)[0] + b" ":
            return None
        return json.loads(body.decode())["ip"]
    except (OSError, ValueError, KeyError):
        return None


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


def live_check(controller_addr, secret, port, expected, services):
    payload = json.dumps(
        {"controller": controller_addr, "secret": secret, "port": port, "expected_ip": expected, "services": services}
    )
    code, out = run_cargo(
        ["test", "--manifest-path", str(MANIFEST), TEST_NAME, "--", "--ignored", "--exact", "--nocapture", "--test-threads=1"],
        {"MIHOMO_LIVE_INPUT": payload},
        900,
    )
    if code != 0 or "test result: ok. 1 passed" not in out:
        return None, f"实测用例未成功运行（exit={code}）：{out[-600:]}"
    for line in out.splitlines():
        if "MIHOMO_RESULT " in line:
            try:
                return json.loads(line.split("MIHOMO_RESULT ", 1)[1]), ""
            except ValueError:
                return None, "结果不是 JSON"
    return None, "实测用例没有输出结果"


def blocked(result):
    return result["verdict"] in ("block", "unknown") and bool(result.get("reason"))


def passed_with_a(result):
    return result["verdict"] == "pass" and result.get("ip") == IP_A and result.get("mode") == "rule" and bool(result.get("reason"))


# ----- 主流程 -----
def main():
    sys.stdout.reconfigure(encoding="utf-8")
    tmp = Path(tempfile.mkdtemp(prefix="mihomo-harness-"))
    results = {}
    notes = {}
    upstream_a = upstream_b = mihomo = None
    try:
        p_a, p_b, p_mixed, p_ctl, p_claude, p_none = free_ports(6)
        upstream_a = start_upstream(p_a, {"*": IP_A, "odd.test": IP_A_ODD})
        upstream_b = start_upstream(p_b, {"*": IP_B})
        (tmp / "config.yaml").write_text(
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
                    "find-process-mode: off",
                    "profile: {store-selected: false, store-fake-ip: false}",
                    "dns: {enable: false}",
                    "tun: {enable: false}",
                    "proxies:",
                    f"  - {{name: A, type: http, server: 127.0.0.1, port: {p_a}}}",
                    f"  - {{name: B, type: http, server: 127.0.0.1, port: {p_b}}}",
                    "proxy-groups:",
                    "  - {name: PROXY, type: select, proxies: [A, B]}",
                    "rules:",
                    "  - MATCH,PROXY",
                    "listeners:",
                    f"  - {{name: claude, type: mixed, listen: 127.0.0.1, port: {p_claude}, proxy: A}}",
                    "",
                ]
            ),
            encoding="utf-8",
        )
        ctl = f"127.0.0.1:{p_ctl}"

        # ----- 自检 -----
        problems = []
        if not MIHOMO.is_file():
            problems.append("找不到 verge-mihomo.exe")
        else:
            v = subprocess.run([str(MIHOMO), "-v"], capture_output=True, timeout=30)
            if VERSION not in v.stdout.decode("utf-8", "replace").split():
                problems.append(f"内核版本不是 {VERSION}")
        if ask(p_a, "http://svc1.test/") != IP_A or ask(p_b, "http://svc1.test/") != IP_B or IP_A == IP_B:
            problems.append("A、B 直连没有回不同 IP")
        if ask(p_a, "http://odd.test/") != IP_A_ODD:
            problems.append("A 不能按 Host 回不同 IP")
        if not problems:
            log = open(tmp / "mihomo.log", "wb")
            mihomo = subprocess.Popen([str(MIHOMO), "-d", str(tmp), "-f", str(tmp / "config.yaml")], cwd=tmp, stdout=log, stderr=subprocess.STDOUT)
            CHILDREN.append(mihomo)
            for _ in range(100):
                if controller(p_ctl, "GET", "/version")[0] == 200:
                    break
                time.sleep(0.2)
            status, body = controller(p_ctl, "GET", "/version")
            if status != 200 or VERSION not in body.decode("utf-8", "replace"):
                problems.append("独立 mihomo 未就绪或版本不符")
            elif ask(p_claude, "http://svc1.test/") != IP_A:
                problems.append("专用端口没有走 A")
            elif controller(p_ctl, "GET", "/version", secret="wrong")[0] != 401:
                problems.append("错误 secret 未被拒绝")
        if not problems:
            code, out = run_cargo(["test", "--manifest-path", str(MANIFEST), "--no-run"], {}, 1800)
            if code != 0:
                problems.append(f"cargo test 编译失败：{(out or '')[-600:]}")
        results[SCENARIOS[0]] = not problems
        notes[SCENARIOS[0]] = "；".join(problems)

        if results[SCENARIOS[0]]:

            def scenario(name, ok_fn, *args, extra=None):
                hits_b = upstream_b.hits
                result, err = live_check(*args)
                if result is None:
                    results[name], notes[name] = False, err
                    return
                ok = ok_fn(result)
                if ok and extra:
                    ok = extra(hits_b)
                results[name] = ok
                notes[name] = json.dumps(result, ensure_ascii=False)

            no_b = lambda before: upstream_b.hits == before
            scenario(SCENARIOS[1], passed_with_a, ctl, SECRET, p_claude, IP_A, SERVICES, extra=no_b)

            # 日常组切到 B，并用 mixed-port 证明切换真的生效
            switched = controller(p_ctl, "PUT", "/proxies/PROXY", {"name": "B"})[0] == 204 and ask(p_mixed, "http://svc1.test/") == IP_B
            if switched:
                scenario(SCENARIOS[2], passed_with_a, ctl, SECRET, p_claude, IP_A, SERVICES, extra=no_b)
            else:
                results[SCENARIOS[2]], notes[SCENARIOS[2]] = False, "日常组切换未生效"

            scenario(SCENARIOS[5], blocked, ctl, "wrong-secret", p_claude, IP_A, SERVICES)
            scenario(SCENARIOS[6], blocked, ctl, SECRET, p_claude, "192.0.2.77", SERVICES)
            scenario(SCENARIOS[7], blocked, ctl, SECRET, p_claude, IP_A, SERVICES_ODD)
            scenario(SCENARIOS[8], blocked, ctl, SECRET, p_none, IP_A, SERVICES)

            # 日常组切回 A：此时经 mixed-port 出口也是 A，只有端口规则能拦住
            back = controller(p_ctl, "PUT", "/proxies/PROXY", {"name": "A"})[0] == 204 and ask(p_mixed, "http://svc1.test/") == IP_A
            if back:
                scenario(SCENARIOS[9], blocked, ctl, SECRET, p_mixed, IP_A, SERVICES)
            else:
                results[SCENARIOS[9]], notes[SCENARIOS[9]] = False, "日常组切回 A 未生效"

            # A 挂：先确认专用端口确实不通，再判
            stop_upstream(upstream_a)
            upstream_a = None
            if ask(p_claude, "http://svc1.test/") is None:
                scenario(SCENARIOS[3], blocked, ctl, SECRET, p_claude, IP_A, SERVICES)
            else:
                results[SCENARIOS[3]], notes[SCENARIOS[3]] = False, "A 停了专用端口仍有出口"

            kill_tree(mihomo)
            if controller(p_ctl, "GET", "/version")[0] is None:
                scenario(SCENARIOS[4], blocked, ctl, SECRET, p_claude, IP_A, SERVICES)
            else:
                results[SCENARIOS[4]], notes[SCENARIOS[4]] = False, "mihomo 未停止"
    finally:
        for proc in CHILDREN:
            kill_tree(proc)
        for server in (upstream_a, upstream_b):
            if server is not None:
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
