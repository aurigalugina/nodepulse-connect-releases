use std::path::PathBuf;
use tauri_plugin_shell::ShellExt;
use serde::{Deserialize, Serialize};

/// Locate the tailscale.exe binary on Windows.
/// Checks: Program Files, Program Files (x86), then PATH.
#[tauri::command]
pub async fn detect_tailscale() -> Result<String, String> {
    let candidates = vec![
        r"C:\Program Files\Tailscale\tailscale.exe",
        r"C:\Program Files (x86)\Tailscale\tailscale.exe",
    ];

    for path in &candidates {
        if PathBuf::from(path).exists() {
            return Ok(path.to_string());
        }
    }

    // Fallback: check PATH via `where tailscale`
    let output = std::process::Command::new("where")
        .arg("tailscale")
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if !path.is_empty() {
            return Ok(path);
        }
    }

    Err("Tailscale not found. Please install Tailscale for Windows.".to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TailscaleStatus {
    pub online: bool,
    pub mesh_ip: Option<String>,
    pub backend_state: String,
}

/// Run `tailscale status --json` and return parsed status.
#[tauri::command]
pub async fn tailscale_status(app: tauri::AppHandle) -> Result<TailscaleStatus, String> {
    let tailscale_path = detect_tailscale().await?;

    let output = app
        .shell()
        .command(&tailscale_path)
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

    let backend_state = raw["BackendState"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();

    let self_online = raw["Self"]["Online"].as_bool().unwrap_or(false);

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

/// Run `tailscale up` with the provided auth key and hostname.
/// Returns stdout+stderr output for UI progress display.
#[tauri::command]
pub async fn tailscale_up(
    app: tauri::AppHandle,
    login_server: String,
    authkey: String,
    hostname: String,
) -> Result<String, String> {
    let tailscale_path = detect_tailscale().await?;

    let output = app
        .shell()
        .command(&tailscale_path)
        .args([
            "up",
            "--login-server",
            &login_server,
            "--authkey",
            &authkey,
            "--hostname",
            &hostname,
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

/// Disconnect from the mesh: `tailscale logout`.
#[tauri::command]
pub async fn tailscale_down(app: tauri::AppHandle) -> Result<(), String> {
    let tailscale_path = match detect_tailscale().await {
        Ok(p) => p,
        Err(_) => return Ok(()), // already gone / not installed
    };

    app.shell()
        .command(&tailscale_path)
        .arg("logout")
        .output()
        .await
        .map_err(|e| format!("Failed to run tailscale logout: {e}"))?;

    Ok(())
}
