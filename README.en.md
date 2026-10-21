# CC-Guard

**Keep Claude on the one line you chose in Clash, and switch between Claude accounts without mixing them up.**

A Windows desktop app for people who already run [Clash Verge Rev](https://github.com/clash-verge-rev/clash-verge-rev) (mihomo core). CC-Guard does not set up proxies or touch your subscription. It **checks** that the dedicated line really works, **gates** every Claude launch, and **manages** the local data of several Claude accounts.

[中文](README.md) · [Changelog](CHANGELOG.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)

![Accounts page with line status, account cards and launch buttons](docs/images/status-pass.png)

> v0.3.0, under active development. Windows 10/11 x64 only. Accounts and IPs in screenshots are test fixtures.

## Features

| Feature | What it does |
|---|---|
| Line check | Asks several independent exit-lookup services through your Claude-only port. It passes only when at least two answer, all answers agree, and the IP equals the exit you saved. |
| Launch gate | Re-checks the line before starting Claude Code, Claude Desktop or the web version; refuses with a reason when it does not pass. |
| Claude-only proxy | Claude Code gets the dedicated port only in its own console; Claude Desktop starts with `--proxy-server`. System proxy and your environment variables are left alone. |
| Connection check | Reads Clash's live connections and lists Claude traffic (matched by domain and process) that did not pass through the chosen node or group. |
| Accounts | Each account's sign-in state and local data are stored encrypted and separately; the current account is saved before switching, with rollback on failure. |
| Setup wizard | Connect to Clash → pick the dedicated port and node → check and remember the exit. |

It does **not** build proxies, fake your IP or location, or promise how the service will treat your account.

## Requirements

1. Clash Verge Rev with **External controller** enabled (Settings → Clash settings), e.g. `127.0.0.1:9097`, plus its secret. Clash Verge only opens a named pipe by default.
2. A Claude-only listener bound to the node or group Claude should use, added through Clash Verge's global Merge override:

   ```yaml
   listeners:
     - name: claude-fixed-http
       type: http
       port: 17892
       listen: 127.0.0.1
       proxy: YOUR-CLAUDE-NODE-OR-GROUP
   ```

   Recommended rules so Claude traffic outside that port (for example under TUN) goes the same way:

   ```yaml
   prepend-rules:
     - PROCESS-NAME,claude.exe,YOUR-CLAUDE-NODE-OR-GROUP
     - DOMAIN-SUFFIX,anthropic.com,YOUR-CLAUDE-NODE-OR-GROUP
     - DOMAIN-SUFFIX,claude.ai,YOUR-CLAUDE-NODE-OR-GROUP
     - DOMAIN-SUFFIX,claude.com,YOUR-CLAUDE-NODE-OR-GROUP
   ```

3. Claude Code and/or Claude Desktop from the official channels.

## Install

No release yet; build from source (see Development). The result is `CC-Guard_<version>_x64-setup.exe`, a per-user install without admin rights. Start CC-Guard from the Start menu or Explorer, **not from a terminal**: it would pass the terminal's environment on to Claude Code.

## How it decides

- Line check: read-only `/configs`, then ifconfig.co, Cloudflare trace and ipwho.is through the dedicated port. A port equal to one of Clash's general inbounds (mixed-port etc.) fails, because it follows everyday routing.
- Connection check: read-only `/connections`. A Claude connection is fine when its chain contains the chosen node or group, so fallback switches inside a group are not false alarms; the exit IP itself is still verified by the line check.
- Unreadable, timed out or malformed results never pass. Only a pass is shown in green.

## Privacy

Settings and the account vault live in `%APPDATA%\io.github.shiqiyux-cmd.cc-guard`, readable only by your Windows account. The Clash secret is write-only for the UI. Claude credentials stay in the native layer and never reach the UI or the network. No telemetry; the only outside requests are the three exit-lookup services, which see your exit IP. See [SECURITY.md](SECURITY.md).

## Known limits

- Only connections that pass through Clash are visible; without TUN, direct connections that bypass the proxy are not.
- Results describe the moment of the check; the tray re-checks every 5 minutes.
- The web version still uses the bundled Fingerprint Chromium (~130 MB installer); moving to a separate Edge/Chrome profile is planned.
- The installer is not code-signed; SmartScreen may warn.

## Development

Rust (x64 MSVC), Visual Studio C++ Build Tools, WebView2, Bun, Node.js 22+, Python 3.12+, PowerShell 7.

```powershell
bun install --frozen-lockfile
bun run tauri dev
bun run build:windows
bun test; bun run test:ui; bun run test:identities
cargo test --manifest-path src-tauri/Cargo.toml
```

## Credits and license

Derived from the identity and browser code of [liseami/CCBaga](https://github.com/liseami/CCBaga) (MIT). Original work is [MIT](LICENSE); third-party notices are in [NOTICE.md](NOTICE.md). Claude and Anthropic are trademarks of Anthropic; this project is not affiliated with Anthropic.
