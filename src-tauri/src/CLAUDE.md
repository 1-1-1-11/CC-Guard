# src-tauri/src/
> L2 | 父级: ../CLAUDE.md
main.rs: Tauri 生命周期、单实例、--browser/--tray 入口；setup 第一步做旧数据迁移；只注册 src/contract.ts 列出的命令。
line.rs / line_tests.rs: 契约里的线路命令。settings.json 经 identities::private 写成本用户独占文件，secret 只写不读、损坏时报错不覆盖，读到早期默认出口服务时换成当前默认。launch 拿共享锁、确认身份恢复已完成、每次重新检查，只有通过才启动：CLI 开新控制台只注入专用端口代理；桌面版经执行别名 claude-desktop.exe 带 --proxy-server 启动，已在运行或找不到别名时拒绝；网页版交给 browser::open_gated。
mihomo/: mod.rs 线路检查（只读 /configs；经专用端口逐个请求 ifconfig.co、Cloudflare trace、ipwho.is，至少 2 个成功且一致并等于期望出口才通过；专用端口等于普通入站端口即拦住；/proxies 节点列表代理组在前）；connections.rs 连接核对（链路里没有指定节点或代理组即漏网，读不到链路也算漏网）；live.rs 只供 tests/mihomo-harness 调起；tests.rs 纯逻辑回归。
migrate.rs: 改名后首次启动，新目录不存在、旧目录 local.proxy.ccbaga 存在且 requirement-french.exe 未运行时整份复制，按本用户独占权限重建；遇链接或特殊文件整体放弃。失败只写 stderr。
tray.rs: 托盘每 5 分钟做一次线路检查，故障和恢复各通知一次；托盘启动与首页共用 line::launch_blocking；release 版首次启动写 HKCU Run（--tray）。
identities/: 身份保险库；见其 CLAUDE.md。
browser/: 指纹浏览器；见其 CLAUDE.md。
cleaner.rs / cleaner_targets.rs / cleaner_identity.rs / cleaner_config.rs / cleaner_folder.rs / cleaner_io.rs / cleaner_tests.rs: 身份切换用的 Claude 进程识别与结束、白名单目标、窄范围配置变换与原子写入；不再注册批量清理命令。
claude_cli.rs: 内嵌 scripts/claude-cli.ps1 的 CLI 只读状态与卸载/重装，Job Object 限定子进程树，成功需完整回报。
system.rs: 电脑状态读取、时区写入与关闭定位。
regions.rs: 地区目录、出口地区匹配与浏览器核验目标。
windows.rs / windows_process.rs / windows_timezones.rs: 当前用户 SID 与文件所有权、时区/语言、Claude 进程与 MSIX 包族识别、CLDR 时区映射。
native.rs / progress.rs / ui_language.rs: JSON 解码与凭据快照接口、有序脱敏进度、zh/en 界面语言。
updater_tests.rs: 上游签名更新器的密码学回归，不代表启用了更新。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
