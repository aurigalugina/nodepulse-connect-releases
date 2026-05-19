use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;
use serde::{Deserialize, Serialize};

// ── Path resolution ────────────────────────────────────────────────────────────

/// Resolve the tailscale binary path.
/// Priority: system install → bundled sidecar.
async fn resolve_tailscale_path(app: &tauri::AppHandle) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        // 1. System install
        let system = [
            r"C:\Program Files\Tailscale\tailscale.exe",
            r"C:\Program Files (x86)\Tailscale\tailscale.exe",
        ];
        for path in &system {
            if PathBuf::from(path).exists() {
                return Ok(path.to_string());
            }
        }

        // 2. PATH
        if let Ok(out) = std::process::Command::new("where").arg("tailscale").output() {
            if out.status.success() {
                let p = String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !p.is_empty() {
                    return Ok(p);
                }
            }
        }

        // 3. Bundled sidecar
        if let Ok(res) = app.path().resource_dir() {
            let bundled = res.join("tailscale.exe");
            if bundled.exists() {
                return Ok(bundled.to_string_lossy().into_owned());
            }
        }

        return Err("Tailscale not found. Install it or use the bundled version.".to_string());
    }

    #[cfg(not(target_os = "windows"))]
    {
        // macOS / Linux: check common locations + PATH
        let candidates = [
            "/usr/local/bin/tailscale",
            "/usr/bin/tailscale",
            "/opt/homebrew/bin/tailscale",
        ];
        for path in &candidates {
            if PathBuf::from(path).exists() {
                return Ok(path.to_string());
            }
        }
        if let Ok(out) = std::process::Command::new("which").arg("tailscale").output() {
            if out.status.success() {
                let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !p.is_empty() {
                    return Ok(p);
                }
            }
        }
        Err("Tailscale not detected. Install Tailscale before connecting.".to_string())
    }
}

// ── Windows: daemon management ─────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn is_tailscaled_running() -> bool {
    // A quick way: try to run `tailscale status` and see if it responds.
    // We probe via the named pipe existence.
    PathBuf::from(r"\\.\pipe\ProtectedPrefix\Administrators\Tailscale\tailscaled").exists()
}

#[cfg(target_os = "windows")]
async fn ensure_tailscaled_running(app: &tauri::AppHandle) -> Result<(), String> {
    if is_tailscaled_running() {
        return Ok(());
    }

    // Find bundled tailscaled.exe
    let tailscaled = match app.path().resource_dir() {
        Ok(res) => {
            let p = res.join("tailscaled.exe");
            if p.exists() { p } else {
                // No bundled daemon — if system tailscale was found, daemon may be running as service.
                // Give it a moment and check once more.
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                if is_tailscaled_running() { return Ok(()); }
                return Err("Tailscale daemon is not running. Please start the Tailscale service or install Tailscale.".to_string());
            }
        }
        Err(e) => return Err(format!("Cannot locate resources: {e}")),
    };

    // Get app data dir for state file
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&data_dir).ok();
    let state_path = data_dir.join("tailscale.state");

    // Launch tailscaled.exe with UAC elevation via PowerShell
    let ts_path = tailscaled.to_string_lossy().replace('\'', "''");
    let st_path  = state_path.to_string_lossy().replace('\'', "''");
    let ps_cmd = format!(
        "Start-Process -FilePath '{ts_path}' -ArgumentList '--state=\"{st_path}\"' -Verb RunAs -WindowStyle Hidden"
    );

    std::process::Command::new("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_cmd])
        .spawn()
        .map_err(|e| format!("Failed to launch tailscaled: {e}"))?;

    // Poll until daemon is ready (up to 10 seconds)
    for _ in 0..20 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        if is_tailscaled_running() {
            return Ok(());
        }
    }

    Err("Tailscale daemon did not start in time. Try running the app as administrator.".to_string())
}

// ── Tauri commands ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn detect_tailscale(app: tauri::AppHandle) -> Result<String, String> {
    resolve_tailscale_path(&app).await
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TailscaleStatus {
    pub online: bool,
    pub mesh_ip: Option<String>,
    pub backend_state: String,
}

#[tauri::command]
pub async fn tailscale_status(app: tauri::AppHandle) -> Result<TailscaleStatus, String> {
    let path = resolve_tailscale_path(&app).await?;

    let output = app
        .shell()
        .command(&path)
        .args(["status", "--json"])
        .output()
        .await
        .map_err(|e| format!("Failed to run tailscale: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "tailscale status error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let raw: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse tailscale status: {e}"))?;

    let backend_state = raw["BackendState"].as_str().unwrap_or("Unknown").to_string();
    let self_online   = raw["Self"]["Online"].as_bool().unwrap_or(false);
    let mesh_ip = raw["Self"]["TailscaleIPs"]
        .as_array()
        .and_then(|ips| ips.first())
        .and_then(|ip| ip.as_str())
        .map(|s| s.to_string());

    Ok(TailscaleStatus {
        online: backend_state == "Running" && self_online,
        mesh_ip,
        backend_state,
    })
}

#[tauri::command]
pub async fn tailscale_up(
    app: tauri::AppHandle,
    login_server: String,
    authkey: String,
    hostname: String,
) -> Result<String, String> {
    // On Windows: ensure daemon is running before issuing commands
    #[cfg(target_os = "windows")]
    ensure_tailscaled_running(&app).await?;

    let path = resolve_tailscale_path(&app).await?;

    let output = app
        .shell()
        .command(&path)
        .args([
            "up",
            "--login-server", &login_server,
            "--authkey",      &authkey,
            "--hostname",     &hostname,
            "--accept-dns=false",
            "--reset",
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run tailscale up: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(if !stderr.is_empty() { stderr } else { stdout });
    }

    Ok(if !stdout.is_empty() { stdout } else { "Connected".to_string() })
}

#[tauri::command]
pub async fn tailscale_down(app: tauri::AppHandle) -> Result<(), String> {
    let path = match resolve_tailscale_path(&app).await {
        Ok(p) => p,
        Err(_) => return Ok(()),
    };

    app.shell()
        .command(&path)
        .arg("logout")
        .output()
        .await
        .map_err(|e| format!("Failed to run tailscale logout: {e}"))?;

    Ok(())
}
