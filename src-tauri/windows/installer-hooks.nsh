; [INPUT]: Tauri NSIS 的当前用户安装目录和固定 CC-Guard-B 名称空间。
; [OUTPUT]: 安装时删除旧版留下的 B 启动项（网页版只从 CC-Guard 主界面经线路检查启动）；卸载时清理本应用注册项。
; [POS]: 安装器窄钩子；不执行 SSH，不删除资料，不改其他浏览器关联。
; [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
!macro NSIS_HOOK_POSTINSTALL
  Delete "$SMPROGRAMS\CC-Guard-B.lnk"
!macroend
!macro NSIS_HOOK_POSTUNINSTALL
  Delete "$SMPROGRAMS\CC-Guard-B.lnk"
  DeleteRegKey HKCU "Software\Clients\StartMenuInternet\CC-Guard-B"
  DeleteRegKey HKCU "Software\Classes\CC-Guard-B.URL"
  DeleteRegKey HKCU "Software\Classes\CC-Guard-B.HTML"
  DeleteRegValue HKCU "Software\RegisteredApplications" "CC-Guard-B"
!macroend
