; NodePulse Connect — NSIS installer hooks

!macro NSIS_HOOK_PREINSTALL
  ; Stop service and kill processes BEFORE extraction so NSIS can overwrite
  ; tailscaled.exe (which the service/process holds a file lock on).
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
  ; Allow tailscaled.exe in/out so WireGuard UDP and DERP relay work.
  nsExec::Exec 'netsh advfirewall firewall delete rule name="NodePulse Connect - Tailscale"'
  Pop $0
  nsExec::Exec 'netsh advfirewall firewall add rule name="NodePulse Connect - Tailscale" dir=out action=allow program="$INSTDIR\tailscaled.exe" enable=yes'
  Pop $0
  nsExec::Exec 'netsh advfirewall firewall add rule name="NodePulse Connect - Tailscale" dir=in action=allow program="$INSTDIR\tailscaled.exe" enable=yes'
  Pop $0

  ; ── Copy wintun.dll to install dir ─────────────────────────────────────────
  ; wintun.dll ships as a bundle resource (placed in $INSTDIR\resources\).
  ; tailscaled loads it from its own directory at startup.
  CopyFiles "$INSTDIR\resources\wintun.dll" "$INSTDIR\wintun.dll"

  ; ── Windows Service setup ───────────────────────────────────────────────────
  ; Write a PowerShell script to a temp file — avoids NSIS quoting hell.
  ; The script creates the NodePulseConnectDaemon service with correct args,
  ; sets permissions so regular users can start/stop it, and sets NO_PROXY.

  FileOpen $9 "$TEMP\np-svc-setup.ps1" w
  FileWrite $9 "param([string]$$instDir)$\n"
  FileWrite $9 "$$dataDir = 'C:\ProgramData\NodePulse Connect\tailscale-state'$\n"
  FileWrite $9 "$\n"
  FileWrite $9 "# Create statedir and grant SYSTEM + Users full access$\n"
  FileWrite $9 "New-Item -ItemType Directory -Force -Path $$dataDir | Out-Null$\n"
  FileWrite $9 "& icacls $$dataDir /grant 'NT AUTHORITY\SYSTEM:(OI)(CI)F' /grant 'BUILTIN\Users:(OI)(CI)F' /T | Out-Null$\n"
  FileWrite $9 "$\n"
  FileWrite $9 "# Remove existing service (idempotent — safe to fail on first install)$\n"
  FileWrite $9 "& sc.exe stop 'NodePulseConnectDaemon' 2>&1 | Out-Null$\n"
  FileWrite $9 "Start-Sleep -Seconds 1$\n"
  FileWrite $9 "& sc.exe delete 'NodePulseConnectDaemon' 2>&1 | Out-Null$\n"
  FileWrite $9 "Start-Sleep -Seconds 1$\n"
  FileWrite $9 "$\n"
  FileWrite $9 "# Build binPath: binary must be double-quoted separately from its args$\n"
  ; The binPath line uses PS string concatenation to avoid nested quoting:
  ; result: '"C:\...\tailscaled.exe" --socket \\.\pipe\... --statedir "..." --state "..."'
  FileWrite $9 "$$bp = ('$\"' + $$instDir + '\tailscaled.exe$\"' +$\n"
  FileWrite $9 "        ' --socket \\.\pipe\NodePulseConnect-tailscaled' +$\n"
  FileWrite $9 "        ' --statedir $\"' + $$dataDir + '$\"' +$\n"
  FileWrite $9 "        ' --state $\"' + $$dataDir + '\tailscale.state$\"')$\n"
  FileWrite $9 "$\n"
  FileWrite $9 "& sc.exe create 'NodePulseConnectDaemon' binPath= $$bp start= demand obj= LocalSystem DisplayName= 'NodePulse Connect Daemon'$\n"
  FileWrite $9 "$\n"
  FileWrite $9 "# Allow Authenticated Users to start/stop/query the service (no admin needed)$\n"
  FileWrite $9 "# SDDL breakdown: SY=SYSTEM full, BA=Admins full, AU=AuthUsers start+stop+query$\n"
  FileWrite $9 "& sc.exe sdset 'NodePulseConnectDaemon' 'D:(A;;CCLCSWRPWPDTLOCRRC;;;SY)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;CCLCSWRPLO;;;AU)'$\n"
  FileWrite $9 "$\n"
  FileWrite $9 "# Set NO_PROXY so the daemon skips WinHTTP proxy detection (avoids stalling doLogin)$\n"
  FileWrite $9 "Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\NodePulseConnectDaemon' ``$\n"
  FileWrite $9 "    -Name Environment -Value @('NO_PROXY=*', 'no_proxy=*') -Type MultiString$\n"
  FileWrite $9 "$\n"
  FileWrite $9 "Write-Host 'NodePulseConnectDaemon service installed successfully.'$\n"
  FileClose $9

  nsExec::ExecToLog 'powershell.exe -NonInteractive -ExecutionPolicy Bypass -File "$TEMP\np-svc-setup.ps1" -instDir "$INSTDIR"'
  Pop $0
  Delete "$TEMP\np-svc-setup.ps1"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Stop and delete the service before uninstalling files.
  nsExec::Exec 'sc.exe stop "NodePulseConnectDaemon"'
  Pop $0
  Sleep 3000
  nsExec::Exec 'sc.exe delete "NodePulseConnectDaemon"'
  Pop $0
  Sleep 1000
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Clean up firewall rules.
  nsExec::Exec 'netsh advfirewall firewall delete rule name="NodePulse Connect - Tailscale"'
  Pop $0
!macroend
