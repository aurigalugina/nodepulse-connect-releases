; NodePulse Connect — NSIS installer hooks

!macro NSIS_HOOK_PREINSTALL
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
  ; ── Firewall ────────────────────────────────────────────────────────────────
  nsExec::Exec 'netsh advfirewall firewall delete rule name="NodePulse Connect - Tailscale"'
  Pop $0
  nsExec::Exec 'netsh advfirewall firewall add rule name="NodePulse Connect - Tailscale" dir=out action=allow program="$INSTDIR\tailscaled.exe" enable=yes'
  Pop $0
  nsExec::Exec 'netsh advfirewall firewall add rule name="NodePulse Connect - Tailscale" dir=in action=allow program="$INSTDIR\tailscaled.exe" enable=yes'
  Pop $0

  ; ── wintun.dll ──────────────────────────────────────────────────────────────
  IfFileExists "$INSTDIR\resources\wintun.dll" 0 wintun_already_in_place
    CopyFiles "$INSTDIR\resources\wintun.dll" "$INSTDIR\wintun.dll"
  wintun_already_in_place:

  ; ── Statedir (NO SPACES — args after binary in ImagePath are unquoted) ─────────
  CreateDirectory "C:\ProgramData\NodePulseConnect"
  CreateDirectory "C:\ProgramData\NodePulseConnect\ts"
  nsExec::Exec 'icacls "C:\ProgramData\NodePulseConnect" /grant "BUILTIN\Users:(OI)(CI)F" /T /Q'
  Pop $0

  ; ── Windows Service ─────────────────────────────────────────────────────────
  ; Remove old service first.
  nsExec::Exec 'sc.exe stop "NodePulseConnectDaemon"'
  Pop $0
  Sleep 1500
  nsExec::Exec 'sc.exe delete "NodePulseConnectDaemon"'
  Pop $0
  Sleep 1000

  ; Create service entry with placeholder binPath — avoids GetShortPathName
  ; (not a built-in NSIS 3 instruction) and quoting hell in sc.exe.
  nsExec::ExecToLog 'sc.exe create NodePulseConnectDaemon start= demand obj= LocalSystem DisplayName= "NodePulse Connect Daemon" binPath= "C:\PLACEHOLDER"'
  Pop $0

  ; Overwrite ImagePath via registry — NSIS WriteRegExpandStr handles spaces
  ; in $INSTDIR (e.g. "C:\Program Files\NodePulse Connect") with proper quoting.
  ; Resulting ImagePath: "C:\Program Files\NodePulse Connect\tailscaled.exe" --args...
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Services\NodePulseConnectDaemon" "ImagePath" `"$INSTDIR\tailscaled.exe" --socket \\.\pipe\NodePulseConnect-tailscaled --statedir C:\ProgramData\NodePulseConnect\ts --state C:\ProgramData\NodePulseConnect\ts\ts.state`

  ; Grant Authenticated Users start/stop/query (no admin needed at runtime).
  nsExec::Exec 'sc.exe sdset "NodePulseConnectDaemon" "D:(A;;CCLCSWRPWPDTLOCRRC;;;SY)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCSWRPLO;;;AU)"'
  Pop $0

  ; Set NO_PROXY so daemon skips WinHTTP proxy detection.
  nsExec::ExecToLog "powershell -NonInteractive -Command $\"Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\NodePulseConnectDaemon' -Name Environment -Value @('NO_PROXY=*','no_proxy=*') -Type MultiString$\""
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
