; NodePulse Connect — NSIS installer hooks

!macro NSIS_HOOK_PREINSTALL
  ; Stop service + kill processes BEFORE extraction so NSIS can overwrite
  ; tailscaled.exe (the service holds a file lock on it while running).
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
  ; Tauri v2 NSIS places resources in $INSTDIR\resources\. tailscaled loads
  ; wintun.dll from its own directory, so copy it to $INSTDIR.
  IfFileExists "$INSTDIR\resources\wintun.dll" 0 wintun_already_in_place
    CopyFiles "$INSTDIR\resources\wintun.dll" "$INSTDIR\wintun.dll"
  wintun_already_in_place:

  ; ── Windows Service ─────────────────────────────────────────────────────────
  ; Write a PowerShell setup script. Uses New-Service (calls CreateService Win32
  ; API, so SCM registers it immediately — no reboot needed). Uses PowerShell's
  ; -f format operator to build the binPath, avoiding nested quoting issues.
  ;
  ; NSIS escaping in FileWrite double-quoted strings:
  ;   $$   → literal $   (needed for PowerShell variables)
  ;   $\"  → literal "   (needed for quotes inside PS strings)
  ;   $\n  → newline
  FileOpen $9 "$TEMP\np-svc.ps1" w
  FileWrite $9 "param([string]$$instDir)$\n"
  FileWrite $9 "$$d = 'C:\ProgramData\NodePulse Connect\tailscale-state'$\n"
  FileWrite $9 "New-Item -ItemType Directory -Force -Path $$d | Out-Null$\n"
  FileWrite $9 "& icacls $$d /grant 'BUILTIN\Users:(OI)(CI)F' /T | Out-Null$\n"
  FileWrite $9 "& sc.exe stop  NodePulseConnectDaemon 2>&1 | Out-Null$\n"
  FileWrite $9 "Start-Sleep 1$\n"
  FileWrite $9 "& sc.exe delete NodePulseConnectDaemon 2>&1 | Out-Null$\n"
  FileWrite $9 "Start-Sleep 1$\n"
  FileWrite $9 "$$exe = Join-Path $$instDir 'tailscaled.exe'$\n"
  ; Build binPath using -f operator: {0}=exe path (may have spaces), {1}=statedir
  ; Result: '"C:\...\tailscaled.exe" --socket ... --statedir "C:\..." --state "..."'
  FileWrite $9 "$$bp = '$\"{0}$\" --socket \\.\pipe\NodePulseConnect-tailscaled --statedir $\"{1}$\" --state $\"{1}\tailscale.state$\"' -f $$exe, $$d$\n"
  FileWrite $9 "New-Service -Name NodePulseConnectDaemon -BinaryPathName $$bp -StartupType Manual -DisplayName 'NodePulse Connect Daemon' | Out-Null$\n"
  FileWrite $9 "& sc.exe sdset NodePulseConnectDaemon 'D:(A;;CCLCSWRPWPDTLOCRRC;;;SY)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCSWRPLO;;;AU)'$\n"
  FileWrite $9 "Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\NodePulseConnectDaemon' -Name Environment -Value @('NO_PROXY=*','no_proxy=*') -Type MultiString$\n"
  FileWrite $9 "Write-Host 'NodePulseConnectDaemon installed OK'$\n"
  FileClose $9

  nsExec::ExecToLog "powershell.exe -NonInteractive -ExecutionPolicy Bypass -File $\"$TEMP\np-svc.ps1$\" -instDir $\"$INSTDIR$\""
  Pop $0
  Delete "$TEMP\np-svc.ps1"
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
