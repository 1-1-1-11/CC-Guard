<!--
[INPUT]: Windows 原生实现、NSIS 安装配置与本机实测。
[OUTPUT]: Windows 上的安装位置、平台行为与构建说明。
[POS]: README 的 Windows 细节；只写已实现并验证过的行为。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
-->
# Windows 说明

## 安装位置

| 内容 | 位置 |
|---|---|
| 程序 | `%LOCALAPPDATA%\CC-Guard\`（当前用户安装，不需要管理员） |
| 设置与账号保险库 | `%APPDATA%\io.github.shiqiyux-cmd.cc-guard\` |
| 开机项 | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 下的 `CC-Guard`（带 `--tray`，托盘菜单可关闭） |

从旧版 CCBaga Proxy 升级时，第一次启动会把 `%APPDATA%\local.proxy.ccbaga` 整份复制过来；旧程序 `requirement-french.exe` 还在运行时会跳过，先从托盘退出旧程序再启动 CC-Guard。

## 启动方式

| 入口 | 做法 |
|---|---|
| Claude Code | 在工作目录开新控制台运行 `claude`；只给这个控制台设 `HTTP_PROXY`/`HTTPS_PROXY`=专用端口、`NO_PROXY=localhost,127.0.0.1`，并去掉 `ALL_PROXY` |
| 桌面版（MSIX） | 经执行别名 `%LOCALAPPDATA%\Microsoft\WindowsApps\claude-desktop.exe` 带 `--proxy-server=http://127.0.0.1:<专用端口>` 启动；桌面版已在运行时不重复启动 |
| 网页版 | 内置指纹浏览器，代理固定为专用端口，时区跟随出口 |

## 其他平台行为

| 操作 | 实现 |
|---|---|
| 对齐时区 | 把出口时区映射到 Windows 时区 ID，`tzutil /s` 写入后读回实际值 |
| 关闭定位 | 打开 Windows 隐私设置，状态读不到时保持未知 |
| Claude Code 卸载/重装 | 识别官方原生、npm/bun、WinGet 安装，沿用原安装方式并核验版本；保留配置、会话、技能和项目 |
| 进程识别 | 当前用户 Toolhelp + MSIX 包族 `Claude_pzs8sxrjxfjjc`；结束进程前核验 SID、程序、创建时间 |

## 构建

需要 Rust x64 MSVC 工具链、Visual Studio C++ Build Tools、WebView2、Bun、Node.js 22+、Python 3.12+、PowerShell 7。参考 [Tauri 的 Windows 前置条件](https://v2.tauri.app/start/prerequisites/#windows)。

```powershell
bun install --frozen-lockfile
bun run build:windows
```

构建前 `scripts/package-browser.ps1` 会按固定 SHA-256 下载并核验 Fingerprint Chromium 引擎到 `browser-runtime/WindowsBrowser`（不进 Git）。安装包在 `src-tauri/target/release/bundle/nsis/`；设了 `CARGO_TARGET_DIR` 则在对应目录下。

安装包没有 Authenticode 签名，SmartScreen 可能提示；不要为安装而关闭系统安全策略。
