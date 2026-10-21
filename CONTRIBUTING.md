# 参与开发

欢迎 issue 和 PR。动手前请先读根目录和相关模块的 `CLAUDE.md`：它们描述每个文件的职责和边界，改了文件请同步更新。

## 环境

Windows 10/11 x64，Rust（x64 MSVC）、Visual Studio C++ Build Tools、WebView2、Bun、Node.js 22+、Python 3.12+、PowerShell 7。

```powershell
bun install --frozen-lockfile
bun run tauri dev
```

## 提交 PR 前跑这些

```powershell
bun test                  # 前端纯逻辑
bun run build             # 类型检查 + 构建
bun run test:ui           # 界面模拟（Edge，假命令，不碰真实数据）
bun run test:identities   # 账号页模拟
cargo test --manifest-path src-tauri/Cargo.toml
```

改了线路检查或连接核对（`src-tauri/src/mihomo/`），还要跑两套离线判卷台。它们用真实 mihomo 内核加本地假上游，只监听 127.0.0.1，不访问外网、不碰你正在运行的 Clash：

```powershell
$env:MIHOMO_BIN = 'C:\path\to\mihomo.exe'   # 默认 D:\clash_verge\verge-mihomo.exe
py -I tests/mihomo-harness/run.py           # 线路检查
py -I tests/mihomo-harness/connections.py   # 连接核对
```

判卷台第一项是自检，会核对内核版本（当前按 v1.19.32 编写）。

## 规则

- 前后端命令以 [`src/contract.ts`](src/contract.ts) 为准，改契约要前后端一起改。
- 读不到、超时、格式不对，只能判「拦住」或「无法确认」，不能判「通过」；只有「通过」显示绿色。
- 本机权限和进程操作放在 Rust；界面文案同时写进 `src/locales/zh.yaml` 和 `en.yaml`。
- 不放宽测试来让它变绿；改了判断逻辑，请加一条能先失败再通过的测试。
- 不新增依赖，除非确有必要并在 PR 里说明。
- 原创贡献采用 [MIT](LICENSE) 许可；保留第三方许可声明。

## 报告问题

用 issue 模板，写清 Windows 版本、CC-Guard 版本、Clash Verge 版本、做了什么、看到了什么。**不要**贴密钥、订阅地址、节点服务器地址、Claude 登录信息或个人文件内容。安全问题请按 [SECURITY.md](SECURITY.md) 私下报告。
