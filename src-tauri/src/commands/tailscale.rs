use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;
use serde::{Deserialize, Serialize};

// ── Path resolution ────────────────────────────────────────────────────────────

async fn resolve_tailscale_path(_app: &tauri::AppHandle) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let system = [
            r"C:\Program Files\Tailscale\tailscale.exe",
            r"C:\Program Files (x86)\Tailscale\tailscale.exe",
        ];
        for path in &system {
            if PathBuf::from(path).exists() {
                return Ok(path.to_string());
            }
        }
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
        return Err(
            "Tailscale not found. Install Tailscale from https://tailscale.com/download/windows"
                .to_string(),
        );
    }

    #[cfg(not(target_os = "windows"))]
    {
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
    let path = resolve_tailscale_path(&app).await?;

    let output = tokio::time::timeout(
        tokio::time::Duration::from_secs(60),
        app.shell()
            .command(&path)
            .args([
                "up",
                "--login-server", &login_server,
                "--authkey",      &authkey,
                "--hostname",     &hostname,
                "--accept-dns=false",
                "--reset",
            ])
            .output(),
    )
    .await
    .map_err(|_| "Timed out after 60s — check Headscale server reachability and DERP relay.".to_string())?
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
