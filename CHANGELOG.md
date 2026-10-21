# 更新记录

格式参照 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

## [0.3.0] - 2026-10-21

第一个 CC-Guard 版本，由 CCBaga 本机整合版改造而来。

### 新增
- 线路检查：经 Claude 专用端口向 ifconfig.co、Cloudflare trace、ipwho.is 查询出口；至少 2 个服务一致且等于记住的出口才通过，个别服务限流不阻断，说法不一致即拦住。
- 连接核对：读取 mihomo 实时连接，Claude 流量（域名或进程名）的链路里没有指定节点或代理组就算漏网；支持 fallback 等代理组。
- 启动门禁：Claude Code、桌面版、网页版每次启动前重新检查线路。
- Claude Code 只在它的控制台里注入专用端口代理；桌面版经 MSIX 执行别名带 `--proxy-server` 启动，已在运行时拒绝重复启动。
- 卡片式界面、三步首次向导、浅色/深色主题、中英文。
- 首次启动把旧版 `local.proxy.ccbaga` 数据整份复制到新目录（不移动、按本用户独占权限重建）。

### 移除
- Proxy-v2 桥接、修复与防火墙保护。
- macOS、Linux 平台代码与打包，VPS/网络 Skill。
- 许可不明的 vendored 组件库与上游品牌素材。

### 已知问题
- 网页版仍用内置指纹浏览器，安装包约 130 MB。
- 迁移失败的原因只写入标准错误，界面上看不到。
