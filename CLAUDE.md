# CC-Guard

Windows 桌面应用：让 Claude 只走用户在 Clash（mihomo）里指定的线路，并管理多个 Claude 账号的本地数据。应用标识 `io.github.shiqiyux-cmd.cc-guard`，当前版本 0.3.0。面向已在用 Clash Verge Rev 的用户；不部署代理、不改订阅。

Tauri 2 + Rust（Windows x64）+ React 19 + TypeScript + Radix UI + Tailwind CSS 4 + lucide-react + Vite；SQLite（bundled）+ ring AES-256-GCM 身份保险库。

<directory>
src/ - 卡片式界面：账号、连接、设置三页，首次向导，顶部线路状态条；见其 CLAUDE.md
src-tauri/ - 原生宿主：线路检查与连接核对（mihomo）、启动门禁、身份保险库、指纹浏览器、Windows 平台桥；见其 CLAUDE.md
tests/ - 界面模拟的假后端页面与 mihomo 离线判卷台；见其 CLAUDE.md
scripts/ - 界面模拟入口、浏览器引擎获取、CLI 管理脚本与 Windows 回归；见其 CLAUDE.md
browser-runtime/ - 网页版用的 Fingerprint Chromium 引擎（构建时下载，不进 Git）与许可证；见其 CLAUDE.md
assets/ - 应用图标源文件；见其 CLAUDE.md
docs/ - Windows 说明与 README 截图；见其 CLAUDE.md
.github/ - Windows 检查工作流、依赖更新、issue/PR 模板；见其 CLAUDE.md
</directory>
<config>
package.json / bun.lock - 前端依赖与脚本（build、build:windows、test、test:ui、test:identities、test:windows）
vite.config.ts - 主窗口与浏览器起始页两个入口，YAML 文案编译，构建时注入版本号
tsconfig.json - 严格类型检查
README.md / README.en.md - 中英文项目入口
CHANGELOG.md - 版本记录
CONTRIBUTING.md / SECURITY.md / NOTICE.md / LICENSE - 贡献、安全、第三方声明、MIT（保留上游 CCBaga 版权）
</config>

## 不可破的规则

- 前后端命令以 `src/contract.ts` 为唯一来源，改契约前后端一起改。
- 读不到、超时、格式不对只能是「拦住」或「无法确认」，不能是「通过」；只有通过显示绿色。
- 线路检查：至少 2 个出口查询服务成功且一致、并等于记住的出口才通过；专用端口等于 Clash 任一普通入站端口即拦住。
- 连接核对：Claude 流量（域名后缀或进程名）的链路里没有指定节点或代理组即漏网。
- 启动前每次重新检查；CLI 只给子进程注入专用端口代理，桌面版经 MSIX 执行别名带 `--proxy-server` 启动；不改系统代理、不改用户环境变量。
- Clash secret 只写不读；Claude 凭据只在原生层流动；不加遥测。
- 测试只用临时目录、虚构账号和本地假上游；不碰用户正在运行的 Clash 和真实账号。
- 从开始菜单或资源管理器启动应用，不从终端启动（会把终端环境变量传给 CLI）。

## 验证

`bun test`、`bun run build`、`bun run test:ui`、`bun run test:identities`、`cargo test --manifest-path src-tauri/Cargo.toml`；改 `src-tauri/src/mihomo/` 另跑 `py -I tests/mihomo-harness/run.py` 与 `connections.py`（需要 mihomo 内核，`MIHOMO_BIN` 指定路径）。

本机专属说明（旧版位置、回退方式、构建目录）放在被 Git 忽略的 `CLAUDE.local.md`。

[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
