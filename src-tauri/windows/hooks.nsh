; RuVox NSIS installer hooks (#211).
;
; The app spawns mpv.exe via tauri-plugin-mpv. When the installer (an
; auto-update, a manual reinstall, or an uninstall of a running app)
; force-kills ruvox-tauri.exe, the exit-time mpv cleanup never runs and
; the orphaned mpv keeps $INSTDIR\mpv\mpv.exe locked — the install then
; fails with "Error opening file for writing". Kill only OUR mpv
; (path-filtered by $INSTDIR): a user's standalone mpv player, if any,
; must not be touched.
;
; NSIS expands $INSTDIR in the backtick string; $$ is a literal dollar,
; so PowerShell's $_ survives as $_.

!macro NSIS_HOOK_KILL_MPV
  nsExec::ExecToStack `powershell -NoProfile -NonInteractive -Command "Get-Process mpv -ErrorAction SilentlyContinue | Where-Object { $$_.Path -like '$INSTDIR*' } | Stop-Process -Force"`
  Pop $0 ; exit code (ignored: best-effort cleanup)
  Pop $1 ; output (ignored)
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro NSIS_HOOK_KILL_MPV
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro NSIS_HOOK_KILL_MPV
!macroend
