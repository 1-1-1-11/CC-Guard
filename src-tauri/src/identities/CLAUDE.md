# identities/
> L2 | 父级: ../CLAUDE.md
本机 Claude 账号的暂存、切换与恢复。完整 settings 归档时剥离 HTTP_PROXY、HTTPS_PROXY、ALL_PROXY、NO_PROXY、BROWSER，清空/恢复时保留当前值。切换/恢复/删除后经浏览器联动并调用 line 复查线路；未完成的恢复由 ensure_settled 阻止启动。
mod.rs: 当前/空/已保存身份与切换、恢复、重命名、直接删除、停止进程、浏览器联动命令；共享 Cleaner 锁；令牌不送界面。
database.rs: SQLite 账号槽位、归档代次、浏览器绑定、HMAC 索引与持久恢复状态；FULL 同步，凭据正文不入库。
store.rs: 先验收归档再切换工作区，失败回滚，恢复检查点，直接删除不新增备份。
archive.rs / compression.rs / codec.rs: tar + Zstd（兼容旧 gzip）归档，ring AES-256-GCM 认证帧，独立归档密钥。
catalog.rs: 本机 HMAC 匹配账号与稳定槽位。
browser_link.rs: 身份到独占浏览器资料的绑定与稳定种子（派生串保留旧值，改了会让已有指纹变化）。
profile.rs: 只读默认 .claude.json 的名称、邮箱、组织白名单。
credentials.rs: 默认 Code 凭据按策略合并，正文只在原生内存。
private.rs: Windows 创建时指定当前用户所有者与独占 DACL、原子写入；拒绝链接、特殊文件和所有者变化。
tests.rs / optimization_tests.rs / browser_link_tests.rs: 临时账号样本的切换、恢复、网络字段保留、损坏与链接拒绝、跨组织隔离、浏览器绑定回归。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
