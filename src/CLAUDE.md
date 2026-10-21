# src/
> L2 | 父级: ../CLAUDE.md
界面只管输入、显示和执行记录；权限、进程和网络都在原生层，每次启动都由原生端重新检查。命令契约见 contract.ts。
contract.ts: 前后端命令契约（命令名、参数、返回 JSON）的唯一来源，含沿用命令 KEPT 与已删除命令 REMOVED。
main.tsx: 主窗口：侧栏（账号、连接，底部设置，CC-Guard 与版本号），顶部状态条，首次向导，Toast；窗口隐藏时停止轮询。
status-bar.tsx: useLineCheck 与顶部状态条：pass 绿、block 红、unknown 灰，显示原因、出口地区、分流模式与「重新检查」。
line-model.ts / line-model.test.ts: 显示用判定（错误、无结果、过期一律 unknown）、首次设置判断、分流模式文案、漏网合并排序。
wizard.tsx: 首次向导（controller 空或 port 为 0 时弹出）：连上 Clash → 选端口和节点 → 检查并记住出口。
settings.tsx: 设置页：线路（controller、secret 只写不回填、专用端口、节点下拉、期望 IP 与「用当前出口」）、通用（语言、主题、CLI 工作目录）、关于（CLI 卸载/重装）。
launch-bar.tsx: 「启动 Claude Code / 桌面版 / 网页版」，调 launch，被拦或出错用 Toast 显示原因并回写检查结果。
connections.tsx: 连接页：判定、Claude 连接计数，漏网标红置顶；process_matching=false 时提示只按域名核对。
identities.tsx: 账号页：账号卡片（使用中 / 切换 / ⋯ 改名删除）、空账号、保留选项、浏览器联动、切换与删除确认。
identity-model.ts: 身份、保留策略、账号显示字段与浏览器绑定类型；不含凭据。
claude-cli-panel.tsx: Claude Code CLI 安装状态与确认后的卸载/重装。
cleaner-activity.tsx / cleaner-model.ts / cleaner-model.test.ts: 身份与 CLI 事务的进度和日志，路径父子去重与规模格式化。
browser-start.tsx / browser-model.ts / browser-network.ts / browser-network.test.ts: 指纹浏览器起始页：真实 navigator/Intl 与出口观测上报，目标地区核验，服务限流时切换备用服务。
theme.ts: 浅色/深色/跟随系统，只存 localStorage。
i18n.ts / i18n.test.ts: 文案翻译与原生消息模板匹配；测试要求中英键一致、界面文案全在 YAML 里。
index.css: 语义色板（pass 是唯一绿色）与 Tailwind 主题。
app-version.d.ts: 构建时注入的 __APP_VERSION__ 声明。
components/ui/: Button、Card、Badge、Dialog（警告默认聚焦取消）、Input、Select、Switch、Checkbox、Tooltip、Toast、DropdownMenu 与 cn；基于 radix-ui。
locales/: 中英文 YAML 文案；见其 CLAUDE.md。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
