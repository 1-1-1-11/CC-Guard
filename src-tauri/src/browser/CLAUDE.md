# browser/
> L2 | 父级: ../CLAUDE.md
网页版用的指纹浏览器。所有启动路径（首页/托盘 launch、--browser 入口、browser_launch、切换身份后重开）都先过 line::browser_gate：身份恢复未完成或线路未通过就不启动；通过后把专用端口写入当前身份配置，并核对已运行资料进程的实际代理参数。每次启动时区跟随本次检查的出口时区、语言 en-US。
mod.rs: 读取与启动命令、互斥锁与运行状态、引擎发现、起始页会话。
launch.rs: 已持有锁时的唯一启动核心。
workspaces.rs: 按 SQLite 绑定解析当前身份的独占资料与稳定种子；切换身份时正常退出、切换、重新过门禁后重开。
config.rs: 指纹配置、代理与语言白名单、跟随出口规则与 Chromium 参数。
profile.rs: 资料目录边界与偏好窄范围写入。
network.rs: 出口服务目录、出口解析（JSON 与 Cloudflare trace 文本）、核验项。
server.rs: 随机令牌的 127.0.0.1 回环服务与静态资源（图片 MIME 白名单）。
windows.rs: Windows 引擎、open_gated 与 --browser 入口。
tests.rs / workspace_tests.rs: 临时目录样本：代理与资料边界、Cookie 隔离、种子稳定、恢复阻断、跟随出口。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
