; NodePulse Connect — NSIS installer hooks

!macro NSIS_HOOK_PREINSTALL
  ; Stop service and kill processes BEFORE extraction so NSIS can overwrite
  ; tailscaled.exe (which the service holds a file lock on).
  nsExec::Exec 'sc.exe stop "NodePulseConnectDaemon"'
  Pop $0
  Sleep 2500
  nsExec::Exec 'taskkill /F /IM tailscaled.exe /T'
  Pop $0
  nsExec::Exec 'taskkill /F /IM "NodePulse Connect.exe" /T'
  Pop $0
  Sleep 1000
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; ── Firewall rules ──────────────────────────────────────────────────────────
  nsExec::Exec 'netsh advfirewall firewall delete rule name="NodePulse Connect - Tailscale"'
  Pop $0
  nsExec::Exec 'netsh advfirewall firewall add rule name="NodePulse Connect - Tailscale" dir=out action=allow program="$INSTDIR\tailscaled.exe" enable=yes'
  Pop $0
  nsExec::Exec 'netsh advfirewall firewall add rule name="NodePulse Connect - Tailscale" dir=in action=allow program="$INSTDIR\tailscaled.exe" enable=yes'
  Pop $0

  ; ── wintun.dll ──────────────────────────────────────────────────────────────
  ; Tauri v2 NSIS places bundle resources under $INSTDIR\resources\.
  ; tailscaled loads wintun.dll from its own directory, so copy it there.
  IfFileExists "$INSTDIR\resources\wintun.dll" 0 wintun_already_in_place
    CopyFiles "$INSTDIR\resources\wintun.dll" "$INSTDIR\wintun.dll"
  wintun_already_in_place:

  ; ── Statedir ────────────────────────────────────────────────────────────────
  ; Create statedir and grant Users full access so the user-mode app can wipe
  ; it on each connect attempt (SYSTEM service writes here, user app reads/wipes).
  CreateDirectory "C:\ProgramData\NodePulse Connect"
  CreateDirectory "C:\ProgramData\NodePulse Connect\tailscale-state"
  nsExec::Exec 'icacls "C:\ProgramData\NodePulse Connect" /grant "BUILTIN\Users:(OI)(CI)F" /T /Q'
  Pop $0

  ; ── Windows Service ─────────────────────────────────────────────────────────
  ; Remove existing service first (idempotent — safe to fail on first install).
  nsExec::Exec 'sc.exe stop "NodePulseConnectDaemon"'
  Pop $0
  Sleep 1500
  nsExec::Exec 'sc.exe delete "NodePulseConnectDaemon"'
  Pop $0
  Sleep 1000

  ; Write service registry entries directly — avoids sc.exe binPath= quoting
  ; issues when $INSTDIR contains spaces (e.g. "NodePulse Connect").
  ; Equivalent to: sc create NodePulseConnectDaemon start=demand obj=LocalSystem
  WriteRegStr     HKLM "SYSTEM\CurrentControlSet\Services\NodePulseConnectDaemon" \
                  "DisplayName" "NodePulse Connect Daemon"
  WriteRegDWORD   HKLM "SYSTEM\CurrentControlSet\Services\NodePulseConnectDaemon" \
                  "Start" 3
  WriteRegDWORD   HKLM "SYSTEM\CurrentControlSet\Services\NodePulseConnectDaemon" \
                  "Type" 16
  WriteRegDWORD   HKLM "SYSTEM\CurrentControlSet\Services\NodePulseConnectDaemon" \
                  "ErrorControl" 1
  WriteRegStr     HKLM "SYSTEM\CurrentControlSet\Services\NodePulseConnectDaemon" \
                  "ObjectName" "LocalSystem"

  ; ImagePath: binary must be quoted (it has spaces), args do not need quoting
  ; because statedir path also has spaces — both are double-quoted.
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Services\NodePulseConnectDaemon" \
    "ImagePath" \
    "$\"$INSTDIR\tailscaled.exe$\" --socket \\.\pipe\NodePulseConnect-tailscaled --statedir $\"C:\ProgramData\NodePulse Connect\tailscale-state$\" --state $\"C:\ProgramData\NodePulse Connect\tailscale-state\tailscale.state$\""

  ; NO_PROXY: skip WinHTTP proxy detection that stalls doLogin on corporate networks.
  ; WriteRegMultiStr requires /REGEDIT5 hex format — use PowerShell instead.
  nsExec::Exec "powershell -NonInteractive -Command $\"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\NodePulseConnectDaemon' -Name Environment -Value @('NO_PROXY=*','no_proxy=*') -Type MultiString$\""
  Pop $0

  ; Grant Authenticated Users start/stop/query — no admin needed at runtime.
  ; SDDL: SY=SYSTEM full, BA=Admins full, AU=AuthUsers start+stop+query
  nsExec::Exec 'sc.exe sdset "NodePulseConnectDaemon" "D:(A;;CCLCSWRPWPDTLOCRRC;;;SY)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCSWRPLO;;;AU)"'
  Pop $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec 'sc.exe stop "NodePulseConnectDaemon"'
  Pop $0
  Sleep 3000
  nsExec::Exec 'sc.exe delete "NodePulseConnectDaemon"'
  Pop $0
  Sleep 1000
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  nsExec::Exec 'netsh advfirewall firewall delete rule name="NodePulse Connect - Tailscale"'
  Pop $0
!macroend
