/**
 * [INPUT]: CC-Guard 原生命令。
 * [OUTPUT]: 前后端命令契约：命令名、参数（camelCase）与返回 JSON 的唯一来源。
 * [POS]: 第 1 轮前后端并行改造的接缝；后端 #[tauri::command] 必须与此一致，改动需两边同时改。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */

/** 通过 / 拦住 / 无法确认。只有 pass 可显示为绿色。 */
export type Verdict = 'pass' | 'block' | 'unknown';

/** 线路设置。secret 只写不读：读取时只返回 secret_set。 */
export type LineSettings = {
  controller: string;          // mihomo external-controller，如 "127.0.0.1:9097"；空串 = 未设置
  secret_set: boolean;
  port: number;                // Claude 专用入口端口；0 = 未设置
  node: string;                // 指定节点名（mihomo /proxies 里的名字）；空串 = 未设置
  expected_ip: string;         // 期望出口 IP；空串 = 未设定
  services: string[];          // 出口查询服务，按域名去重后需 ≥2
  claude_hosts: string[];      // 按域名后缀认 Claude 流量，默认 ["anthropic.com","claude.ai","claude.com"]
  claude_processes: string[];  // 按进程名认 Claude 流量（不分大小写），默认 ["claude.exe"]
};
export type Settings = { line: LineSettings; workspace: string };
export type SettingsPatch = {
  line?: Partial<Omit<LineSettings, 'secret_set'>> & { secret?: string };
  workspace?: never; // 工作目录只能经 workspace_choose 原生对话框修改
};

/** 未设置完整（controller 空或 port 为 0）时 verdict=unknown、reason 说明缺什么。
 *  expected_ip 为空时照常探测：verdict=unknown，ip/country/timezone 填实际观测值，供「记住这个出口」。 */
export type LineCheck = {
  verdict: Verdict; label: string; reason: string;
  mode: string; ip: string; country: string; timezone: string; checked_at: number;
};

/** 漏网 = Claude 流量（域名或进程命中）的链路里没有指定的节点或代理组；node 是实际节点（chains[0]）。 */
export type Leak = { host: string; process: string; inbound: string; node: string };
export type LineConnections = {
  verdict: Verdict; label: string; reason: string;
  total: number; claude: number; leaks: Leak[];
  process_matching: boolean;   // false = Clash 未开 find-process-mode: always，只按域名核对
  checked_at: number;
};

export type LaunchEntry = 'cli' | 'desktop' | 'web';
/** 启动前必须重新检查线路；未通过时 launched=false，check 为本次检查结果。 */
export type LaunchResult = { launched: boolean; reason: string; check: LineCheck };

export type Commands = {
  settings_read: { args: Record<string, never>; result: Settings };
  settings_write: { args: { patch: SettingsPatch }; result: Settings };
  line_check: { args: Record<string, never>; result: LineCheck };
  line_connections: { args: Record<string, never>; result: LineConnections };
  line_nodes: { args: Record<string, never>; result: string[] };   // 读不到时报错（Err 字符串）
  launch: { args: { entry: LaunchEntry }; result: LaunchResult };
  workspace_choose: { args: Record<string, never>; result: string | null };
};

/** 沿用、不改签名的现有命令。 */
export const KEPT = [
  'identities_read', 'identities_browser', 'identities_stop', 'identities_switch', 'identities_rename', 'identities_delete',
  'cleaner_stop', 'claude_cli_read', 'claude_cli_manage',
  'system_read', 'system_set_timezone', 'system_disable_location',
  'browser_read', 'browser_launch', 'ui_language_read', 'ui_language_write',
] as const;

/** 第 1 轮删除的旧命令（Proxy-v2 整合），前后端都不得再出现。 */
export const REMOVED = [
  'proxy_status', 'proxy_check', 'proxy_launch', 'proxy_repair_plan', 'proxy_repair',
  'proxy_workspace_choose', 'proxy_exit_region', 'proxy_account', 'proxy_connections', 'proxy_protect',
] as const;
