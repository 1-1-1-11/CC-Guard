// [INPUT]: Tauri 配置（tauri.conf.json + tauri.windows.conf.json）。
// [OUTPUT]: Tauri 构建元数据与 Windows 资源。
// [POS]: 原生宿主的构建边界；CC-Guard 只构建 Windows。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
fn main() {
    tauri_build::build();
}
