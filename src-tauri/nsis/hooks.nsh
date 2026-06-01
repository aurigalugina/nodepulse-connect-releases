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

  ; ── Statedir (NO SPACES — required for sc.exe binPath quoting) ──────────────
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

  ; Get 8.3 short path of $INSTDIR to eliminate spaces (e.g. "NodePulse Connect" -> "NODEP~1").
  ; Statedir has no spaces, so the full binPath has no spaces — no inner quoting needed.
  GetShortPathName "$INSTDIR" $R0

  ; Create service via sc.exe directly (no PowerShell, no quoting hell).
  ; binPath outer "..." is the whole binary+args string. Since $R0 and statedir have
  ; no spaces, the binary is unambiguous and no inner quotes are required.
  nsExec::ExecToLog "sc.exe create NodePulseConnectDaemon binPath= $\"$R0\tailscaled.exe --socket \\.\pipe\NodePulseConnect-tailscaled --statedir C:\ProgramData\NodePulseConnect\ts --state C:\ProgramData\NodePulseConnect\ts\ts.state$\" start= demand obj= LocalSystem DisplayName= $\"NodePulse Connect Daemon$\""
  Pop $0

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
