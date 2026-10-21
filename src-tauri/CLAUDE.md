# src-tauri/
> L2 | 父级: ../CLAUDE.md
原生宿主，只构建 Windows x64。
src/: Rust 源码；见其 CLAUDE.md。
SECURITY.md: 原生数据流、外部连接与验证范围的安全契约。
Cargo.toml / Cargo.lock: 包名 cc-guard；Tauri、reqwest（rustls）、SQLite bundled、ring、Zstd/tar 等依赖。custom-protocol 由正式打包启用以内嵌前端。
build.rs: Tauri 构建元数据。
tauri.conf.json: 产品名 CC-Guard、标识 io.github.shiqiyux-cmd.cc-guard、版本、窗口、CSP、随包资源；不配置自动更新。
tauri.windows.conf.json: 当前用户 NSIS、内嵌指纹浏览器引擎、Unicode 许可、安装钩子。
windows/: NSIS 安装钩子与 CLDR 许可；见其 CLAUDE.md。
capabilities/: 主窗口能力；见其 CLAUDE.md。
icons/: 应用图标；见其 CLAUDE.md。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
