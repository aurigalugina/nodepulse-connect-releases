use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use serde::{Deserialize, Serialize};

const TAILSCALE_VERSION: &str = "1.66.4";

// ── Daemon state ───────────────────────────────────────────────────────────────

pub struct DaemonHandle {
    pub child: Mutex<Option<std::process::Child>>,
}

impl DaemonHandle {
    pub fn new() -> Self { Self { child: Mutex::new(None) } }

    pub fn kill(&self) {
        if let Ok(mut g) = self.child.lock() {
            if let Some(mut c) = g.take() {
                let _ = c.kill();
                let _ = c.wait();
            }
        }
    }
}

// ── Platform paths ─────────────────────────────────────────────────────────────

fn bin_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path().app_data_dir().expect("app data dir").join("tailscale-bin")
}

#[cfg(windows)]
fn tailscaled_bin(app: &tauri::AppHandle) -> PathBuf { bin_dir(app).join("tailscaled.exe") }
#[cfg(not(windows))]
fn tailscaled_bin(app: &tauri::AppHandle) -> PathBuf { bin_dir(app).join("tailscaled") }

#[cfg(windows)]
fn tailscale_bin(app: &tauri::AppHandle) -> PathBuf { bin_dir(app).join("tailscale.exe") }
#[cfg(not(windows))]
fn tailscale_bin(app: &tauri::AppHandle) -> PathBuf { bin_dir(app).join("tailscale") }

pub fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path().app_data_dir().expect("app data dir").join("tailscale-state")
}

pub fn socket_path(app: &tauri::AppHandle) -> String {
    #[cfg(windows)]
    return r"\\.\pipe\NodePulseConnect\tailscaled".to_string();

    #[cfg(not(windows))]
    data_dir(app).join("tailscaled.sock").to_str().unwrap().to_string()
}

// ── Ready check ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn tailscale_is_ready(app: tauri::AppHandle) -> bool {
    tailscaled_bin(&app).exists() && tailscale_bin(&app).exists()
}

// ── Download URL & subdir per platform ─────────────────────────────────────────

fn build_download_url() -> String {
    let v = TAILSCALE_VERSION;

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    { return format!("https://pkgs.tailscale.com/stable/tailscale_{v}_amd64.tgz"); }

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    { return format!("https://pkgs.tailscale.com/stable/tailscale_{v}_arm64.tgz"); }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    { return format!("https://pkgs.tailscale.com/stable/tailscale_{v}_darwin_arm64.tgz"); }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    { return format!("https://pkgs.tailscale.com/stable/tailscale_{v}_darwin_amd64.tgz"); }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    { return format!("https://pkgs.tailscale.com/stable/tailscale_{v}_windows_amd64.zip"); }

    panic!("unsupported platform")
}

fn extract_subdir() -> String {
    let v = TAILSCALE_VERSION;

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    { return format!("tailscale_{v}_amd64"); }

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    { return format!("tailscale_{v}_arm64"); }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    { return format!("tailscale_{v}_darwin_arm64"); }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    { return format!("tailscale_{v}_darwin_amd64"); }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    { return format!("tailscale_{v}_windows_amd64"); }

    panic!("unsupported platform")
}

// ── Download & extract ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn ensure_tailscale(app: tauri::AppHandle) -> Result<(), String> {
    if tailscaled_bin(&app).exists() && tailscale_bin(&app).exists() {
        return Ok(());
    }

    let dir = bin_dir(&app);
    std::fs::create_dir_all(&dir).map_err(|e| format!("create bin dir: {e}"))?;

    let url = build_download_url();
    let _ = app.emit("tailscale-setup", serde_json::json!({"step": "downloading", "progress": 5}));

    let bytes = reqwest::get(&url)
        .await
        .map_err(|e| format!("download failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("server error: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("read response: {e}"))?;

    let _ = app.emit("tailscale-setup", serde_json::json!({"step": "extracting", "progress": 75}));

    let tmp = dir.join("_extract");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| format!("create tmp dir: {e}"))?;

    #[cfg(not(windows))]
    extract_tgz(&bytes, &tmp)?;
    #[cfg(windows)]
    extract_zip(&bytes, &tmp)?;

    let sub = tmp.join(extract_subdir());

    #[cfg(windows)]
    {
        std::fs::copy(sub.join("tailscale.exe"),  tailscale_bin(&app))
            .map_err(|e| format!("copy tailscale.exe: {e}"))?;
        std::fs::copy(sub.join("tailscaled.exe"), tailscaled_bin(&app))
            .map_err(|e| format!("copy tailscaled.exe: {e}"))?;
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let ts_dst  = tailscale_bin(&app);
        let tsd_dst = tailscaled_bin(&app);
        std::fs::copy(sub.join("tailscale"),  &ts_dst)
            .map_err(|e| format!("copy tailscale: {e}"))?;
        std::fs::copy(sub.join("tailscaled"), &tsd_dst)
            .map_err(|e| format!("copy tailscaled: {e}"))?;
        std::fs::set_permissions(&ts_dst,  std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod tailscale: {e}"))?;
        std::fs::set_permissions(&tsd_dst, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod tailscaled: {e}"))?;
        // Remove quarantine flag on macOS so Gatekeeper allows execution.
        #[cfg(target_os = "macos")]
        for p in &[&ts_dst, &tsd_dst] {
            let _ = std::process::Command::new("xattr")
                .args(["-d", "com.apple.quarantine", p.to_str().unwrap_or("")])
                .status();
        }
    }

    let _ = std::fs::remove_dir_all(&tmp);
    let _ = app.emit("tailscale-setup", serde_json::json!({"step": "done", "progress": 100}));

    // Start daemon immediately so the app can proceed without a relaunch.
    start_daemon(&app);
    Ok(())
}

#[cfg(not(windows))]
fn extract_tgz(bytes: &[u8], dest: &PathBuf) -> Result<(), String> {
    use flate2::read::GzDecoder;
    use tar::Archive;
    let gz = GzDecoder::new(std::io::Cursor::new(bytes));
    let mut archive = Archive::new(gz);
    archive.unpack(dest).map_err(|e| format!("extract tgz: {e}"))
}

#[cfg(windows)]
fn extract_zip(bytes: &[u8], dest: &PathBuf) -> Result<(), String> {
    use zip::ZipArchive;
    let mut archive = ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("open zip: {e}"))?;
    archive.extract(dest).map_err(|e| format!("extract zip: {e}"))
}

// ── Daemon lifecycle ───────────────────────────────────────────────────────────

/// Spawn tailscaled from the app data dir with isolated socket + userspace networking.
/// Non-fatal — silently skips if binary not yet downloaded.
pub fn start_daemon(app: &tauri::AppHandle) {
    let bin = tailscaled_bin(app);
    if !bin.exists() {
        return;
    }

    let state_dir = data_dir(app);
    let _ = std::fs::create_dir_all(&state_dir);
    let socket = socket_path(app);

    // Remove stale socket from a previous run that didn't clean up.
    #[cfg(not(windows))]
    {
        let sock = PathBuf::from(&socket);
        if sock.exists() { let _ = std::fs::remove_file(&sock); }
    }

    match std::process::Command::new(&bin)
        .args([
            "--tun=userspace-networking",
            "--socket", &socket,
            "--statedir", state_dir.to_str().unwrap_or(""),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => {
            // Brief pause so the daemon binds its socket before first CLI call.
            std::thread::sleep(std::time::Duration::from_millis(500));
            if let Ok(mut g) = app.state::<DaemonHandle>().child.lock() {
                *g = Some(child);
            }
        }
        Err(e) => eprintln!("[tailscale] daemon spawn failed: {e}"),
    }
}

// ── CLI helper ─────────────────────────────────────────────────────────────────

async fn run_ts(app: &tauri::AppHandle, args: &[&str]) -> Result<std::process::Output, String> {
    let socket = socket_path(app);
    let mut full: Vec<String> = vec!["--socket".into(), socket];
    full.extend(args.iter().map(|s| s.to_string()));

    tokio::process::Command::new(tailscale_bin(app))
        .args(&full)
        .output()
        .await
        .map_err(|e| format!("tailscale: {e}"))
}

// ── Tauri commands ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct TailscaleStatus {
    pub online: bool,
    pub mesh_ip: Option<String>,
    pub backend_state: String,
}

#[tauri::command]
pub async fn tailscale_status(app: tauri::AppHandle) -> Result<TailscaleStatus, String> {
    let out = run_ts(&app, &["status", "--json"]).await?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    let raw: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("parse status: {e}"))?;

    Ok(TailscaleStatus {
        online: raw["BackendState"].as_str() == Some("Running")
            && raw["Self"]["Online"].as_bool().unwrap_or(false),
        mesh_ip: raw["Self"]["TailscaleIPs"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .map(str::to_string),
        backend_state: raw["BackendState"].as_str().unwrap_or("Unknown").to_string(),
    })
}

#[tauri::command]
pub async fn tailscale_up(
    app: tauri::AppHandle,
    login_server: String,
    authkey: String,
    hostname: String,
) -> Result<String, String> {
    let socket = socket_path(&app);
    let out = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        tokio::process::Command::new(tailscale_bin(&app))
            .args([
                "--socket", &socket,
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
    .map_err(|_| "Timed out after 60s — check Headscale server and DERP relay.".to_string())?
    .map_err(|e| format!("tailscale up: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        return Err(if !stderr.is_empty() { stderr } else { stdout });
    }
    Ok(if !stdout.is_empty() { stdout } else { "Connected".to_string() })
}

#[tauri::command]
pub async fn tailscale_down(app: tauri::AppHandle) -> Result<(), String> {
    let _ = run_ts(&app, &["logout"]).await;
    Ok(())
}
