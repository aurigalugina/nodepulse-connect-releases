; NodePulse Connect — NSIS installer hooks

!macro NSIS_HOOK_PREINSTALL
  ; Kill processes that hold file locks before NSIS extracts new binaries.
  ; Errors ignored — processes may simply not be running.
  nsExec::Exec 'taskkill /F /IM tailscaled.exe /T'
  Pop $0
  nsExec::Exec 'taskkill /F /IM "NodePulse Connect.exe" /T'
  Pop $0
  Sleep 2000
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Add Windows Firewall rules for the bundled tailscaled.exe sidecar.
  ; Without explicit rules, Windows Firewall silently blocks outbound UDP
  ; needed for WireGuard / DERP relay (port 13478) for non-system binaries.
  ; Delete first to avoid duplicate rules on update.
  nsExec::Exec 'netsh advfirewall firewall delete rule name="NodePulse Connect - Tailscale"'
  Pop $0
  nsExec::Exec 'netsh advfirewall firewall add rule name="NodePulse Connect - Tailscale" dir=out action=allow program="$INSTDIR\tailscaled.exe" enable=yes'
  Pop $0
  nsExec::Exec 'netsh advfirewall firewall add rule name="NodePulse Connect - Tailscale" dir=in action=allow program="$INSTDIR\tailscaled.exe" enable=yes'
  Pop $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Clean up firewall rules on uninstall.
  nsExec::Exec 'netsh advfirewall firewall delete rule name="NodePulse Connect - Tailscale"'
  Pop $0
!macroend
