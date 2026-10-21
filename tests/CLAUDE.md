# tests/
> L2 | 父级: ../CLAUDE.md
都不进生产构建，不读本机账号，不访问外网。
app.html / app-fixture.tsx: 按 src/contract.ts 造的内存假后端（secret 只写不读、线路三态、节点、连接快照、启动结果），供 scripts/test-ui.ts。
identities.html / identities-fixture.tsx: 40 个虚构身份与失败/恢复场景，供 scripts/test-identities-ui.ts。
mihomo-harness/run.py: 线路检查判卷台。独立 mihomo 实例（MIHOMO_BIN，默认 D:/clash_verge/verge-mihomo.exe，自检要求 v1.19.32）+ 本地假上游，10 个场景；只监听 127.0.0.1，只结束自己的子进程。
mihomo-harness/connections.py: 连接核对判卷台，同样做法，11 个场景。
两个判卷台被当作判卷标准：修改需说明理由。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
