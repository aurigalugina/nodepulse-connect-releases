; Kill processes that may hold file locks before NSIS extracts new binaries.
; Errors are intentionally ignored — processes may simply not be running.
nsExec::Exec 'taskkill /F /IM tailscaled.exe /T'
Pop $0
nsExec::Exec 'taskkill /F /IM "NodePulse Connect.exe" /T'
Pop $0
Sleep 2000
