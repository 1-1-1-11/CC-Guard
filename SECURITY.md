# 安全说明

## 报告漏洞

请不要在公开 issue 里写漏洞细节。用 GitHub 仓库的 **Security → Report a vulnerability**（私下报告）提交，写清版本、复现步骤和影响。报告里不要附带真实密钥、订阅地址或账号凭据。

只维护最新版本。

## CC-Guard 会碰哪些数据

| 数据 | 位置 | 怎么处理 |
|---|---|---|
| 线路设置（含 Clash 密钥） | `%APPDATA%\io.github.shiqiyux-cmd.cc-guard\settings.json` | 本用户独占权限；密钥只写不读，界面只知道「已设置」 |
| Claude 账号保险库 | 同目录 `identity-vault\` | SQLite 只存目录和恢复状态；凭据与文件归档用 AES-256-GCM 加密，主密钥是本机受保护文件 |
| Claude 登录凭据 | 默认 Claude 配置目录 | 切换账号时由原生层读取和写回，不进入界面、不上传 |
| Clash 实时信息 | 你的 mihomo controller | 只读 `/configs`、`/proxies`、`/connections` |
| 出口 IP | ifconfig.co、Cloudflare trace、ipwho.is | 经专用端口请求，这些服务会看到你的出口 IP |

没有遥测，没有账号资料上传。完整的原生边界见 [src-tauri/SECURITY.md](src-tauri/SECURITY.md)。

## 边界

- 同一个 Windows 用户下的其他程序、以及管理员，理论上能读到这些文件；加密不能防住拥有你账户权限的程序。
- 线路检查只代表检查那一刻；没开 TUN 时，绕过 Clash 的直连看不见。
- 账号切换只管理本地状态，服务端会话失效时仍需重新登录。
