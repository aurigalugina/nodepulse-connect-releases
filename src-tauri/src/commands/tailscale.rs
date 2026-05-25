use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use serde::{Deserialize, Serialize};

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

// ── Binary paths ───────────────────────────────────────────────────────────────

// Linux: Tailscale is downloaded at runtime to app data dir.
#[cfg(target_os = "linux")]
fn bin_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path().app_data_dir().expect("app data dir").join("tailscale-bin")
}

#[cfg(target_os = "linux")]
fn tailscaled_bin(app: &tauri::AppHandle) -> PathBuf { bin_dir(app).join("tailscaled") }
#[cfg(target_os = "linux")]
fn tailscale_bin(app: &tauri::AppHandle) -> PathBuf { bin_dir(app).join("tailscale") }

// Windows / macOS: Tailscale is bundled in the app installer via externalBin.
#[cfg(not(target_os = "linux"))]
fn tailscaled_bin(_app: &tauri::AppHandle) -> PathBuf { bundled_bin("tailscaled") }
#[cfg(not(target_os = "linux"))]
fn tailscale_bin(_app: &tauri::AppHandle) -> PathBuf { bundled_bin("tailscale") }

/// Find a sidecar binary bundled via Tauri externalBin.
/// Dev build  → src-tauri/binaries/<name>-<triple>[.exe]
/// Production → same directory as the main executable (no triple suffix)
#[cfg(not(target_os = "linux"))]
fn bundled_bin(name: &str) -> PathBuf {
    // Dev: binary lives in src-tauri/binaries/ with the full target-triple suffix.
    #[cfg(debug_assertions)]
    {
        let manifest = env!("CARGO_MANIFEST_DIR");
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        let triple_name = format!("{name}-x86_64-pc-windows-msvc.exe");
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let triple_name = format!("{name}-aarch64-apple-darwin");
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        let triple_name = format!("{name}-x86_64-apple-darwin");
        let p = PathBuf::from(manifest).join("binaries").join(&triple_name);
        if p.exists() { return p; }
    }
    // Production: binary is placed next to the main executable (no triple suffix).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            #[cfg(target_os = "windows")]
            let p = dir.join(format!("{name}.exe"));
            #[cfg(not(target_os = "windows"))]
            let p = dir.join(name);
            if p.exists() { return p; }
        }
    }
    #[cfg(target_os = "windows")]
    return PathBuf::from(format!("{name}.exe"));
    #[cfg(not(target_os = "windows"))]
    PathBuf::from(name)
}

// ── Shared paths ───────────────────────────────────────────────────────────────

pub fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path().app_data_dir().expect("app data dir").join("tailscale-state")
}

pub fn socket_path(app: &tauri::AppHandle) -> String {
    #[cfg(target_os = "windows")]
    return r"\\.\pipe\NodePulseConnect-tailscaled".to_string();

    #[cfg(not(target_os = "windows"))]
    data_dir(app).join("tailscaled.sock").to_str().unwrap().to_string()
}

// ── Socket readiness poll ──────────────────────────────────────────────────────

/// Block until the daemon socket/pipe is ready, or 5 s elapsed.
fn wait_for_socket(socket: &str) {
    for _ in 0..25 {
        if socket_is_ready(socket) { return; }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

#[cfg(target_os = "windows")]
fn socket_is_ready(socket: &str) -> bool {
    match std::fs::File::open(socket) {
        Ok(_) => true,
        // ERROR_PIPE_BUSY (231): pipe exists but all instances are occupied — daemon IS running
        Err(e) => e.raw_os_error() == Some(231),
    }
}

#[cfg(not(target_os = "windows"))]
fn socket_is_ready(socket: &str) -> bool {
    std::path::Path::new(socket).exists()
}

// ── Ready check ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn tailscale_is_ready(app: tauri::AppHandle) -> bool {
    tailscaled_bin(&app).exists() && tailscale_bin(&app).exists()
}

// ── ensure_tailscale — Linux: download; Windows/macOS: verify bundle ───────────

#[tauri::command]
pub async fn ensure_tailscale(app: tauri::AppHandle) -> Result<(), String> {
    ensure_impl(app).await
}

// Linux: download Tailscale from pkgs.tailscale.com if not already present.
#[cfg(target_os = "linux")]
async fn ensure_impl(app: tauri::AppHandle) -> Result<(), String> {
    if tailscaled_bin(&app).exists() && tailscale_bin(&app).exists() {
        return Ok(());
    }

    const VER: &str = "1.98.2";
    let dir = bin_dir(&app);
    std::fs::create_dir_all(&dir).map_err(|e| format!("create bin dir: {e}"))?;

    let url = format!("https://pkgs.tailscale.com/stable/tailscale_{VER}_amd64.tgz");
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

    {
        use flate2::read::GzDecoder;
        use tar::Archive;
        let gz = GzDecoder::new(std::io::Cursor::new(&bytes));
        let mut archive = Archive::new(gz);
        archive.unpack(&tmp).map_err(|e| format!("extract tgz: {e}"))?;
    }

    let sub = tmp.join(format!("tailscale_{VER}_amd64"));

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

    let _ = std::fs::remove_dir_all(&tmp);
    let _ = app.emit("tailscale-setup", serde_json::json!({"step": "done", "progress": 100}));

    start_daemon(&app);
    Ok(())
}

// Windows / macOS: binaries are bundled — just verify presence.
#[cfg(not(target_os = "linux"))]
async fn ensure_impl(app: tauri::AppHandle) -> Result<(), String> {
    if tailscaled_bin(&app).exists() && tailscale_bin(&app).exists() {
        Ok(())
    } else {
        Err("Network components not found in application bundle. Please reinstall NodePulse Connect.".to_string())
    }
}

// ── Daemon log ─────────────────────────────────────────────────────────────────

fn daemon_log_path(app: &tauri::AppHandle) -> PathBuf {
    data_dir(app).join("tailscaled.log")
}

fn read_daemon_log(app: &tauri::AppHandle) -> String {
    let path = daemon_log_path(app);
    std::fs::read_to_string(&path).unwrap_or_default()
}

// ── Daemon lifecycle ───────────────────────────────────────────────────────────

/// Spawn tailscaled with isolated socket + userspace networking.
/// Non-fatal — silently skips if binary not yet available or daemon already running.
pub fn start_daemon(app: &tauri::AppHandle) {
    let bin = tailscaled_bin(app);
    if !bin.exists() {
        return;
    }

    // Don't spawn a second daemon if one is still alive.
    if let Ok(mut g) = app.state::<DaemonHandle>().child.lock() {
        if let Some(c) = g.as_mut() {
            match c.try_wait() {
                Ok(None) => return, // process still running
                _ => { *g = None; } // exited or error — fall through to respawn
            }
        }
    }

    let state_dir = data_dir(app);
    let _ = std::fs::create_dir_all(&state_dir);
    let socket = socket_path(app);

    #[cfg(not(target_os = "windows"))]
    {
        let sock = PathBuf::from(&socket);
        if sock.exists() { let _ = std::fs::remove_file(&sock); }
    }

    // Redirect output to log file so we can diagnose failures.
    let log_path = daemon_log_path(app);
    let log_file = std::fs::OpenOptions::new()
        .create(true).write(true).truncate(true)
        .open(&log_path);

    let (stdout_s, stderr_s) = match log_file {
        Ok(f) => {
            let f2 = f.try_clone()
                .or_else(|_| std::fs::OpenOptions::new().append(true).open(&log_path))
                .ok();
            let stderr = f2.map(std::process::Stdio::from)
                .unwrap_or_else(std::process::Stdio::null);
            (std::process::Stdio::from(f), stderr)
        }
        Err(_) => (std::process::Stdio::null(), std::process::Stdio::null()),
    };

    match std::process::Command::new(&bin)
        .args([
            "--tun=userspace-networking",
            "--socket", &socket,
            "--statedir", state_dir.to_str().unwrap_or(""),
            "--state", "mem:",
        ])
        .stdout(stdout_s)
        .stderr(stderr_s)
        .spawn()
    {
        Ok(child) => {
            // Poll until daemon socket is ready — Windows named pipes take longer than Unix sockets.
            wait_for_socket(&socket);
            if let Ok(mut g) = app.state::<DaemonHandle>().child.lock() {
                *g = Some(child);
            }
        }
        Err(e) => {
            let _ = std::fs::write(daemon_log_path(app), format!("daemon spawn failed: {e}"));
            eprintln!("[tailscale] daemon spawn failed: {e}");
        }
    }
}

#[tauri::command]
pub fn get_daemon_log(app: tauri::AppHandle) -> String {
    read_daemon_log(&app)
}

// Kill the daemon process — called before update install so the NSIS extractor
// can overwrite tailscaled.exe (which it holds a file lock on while running).
#[tauri::command]
pub fn stop_daemon(app: tauri::AppHandle) -> Result<(), String> {
    app.state::<DaemonHandle>().kill();
    Ok(())
}

// Start the daemon — called from frontend AFTER the startup update check
// completes, so tailscaled.exe is never running when an update installs.
#[tauri::command]
pub fn launch_daemon(app: tauri::AppHandle) -> Result<(), String> {
    start_daemon(&app);
    Ok(())
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

/// Poll `tailscale status` until the daemon responds (or we run out of iterations).
/// Unlike socket_is_ready, this verifies the daemon can actually process IPC — not just
/// that a named pipe file exists. Each probe has a 3 s hard timeout so a slow daemon
/// cannot inflate the total wait indefinitely.
async fn daemon_can_respond(app: &tauri::AppHandle, max_iter: usize) -> bool {
    for _ in 0..max_iter {
        let probe = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            run_ts(app, &["status", "--json"]),
        ).await;
        match probe {
            Ok(Ok(out)) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stderr.contains("failed to connect to local tailscaled") {
                    return true;
                }
            }
            _ => {} // timeout or error — keep waiting
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    false
}

#[tauri::command]
pub async fn tailscale_up(
    app: tauri::AppHandle,
    login_server: String,
    authkey: String,
    hostname: String,
) -> Result<String, String> {
    let socket = socket_path(&app);

    // Reset strategy: keep the daemon alive if it's already running.
    //
    // Root cause of the persistent "profile data directory: profile not found" bug:
    // tailscale up in v1.98.2 is async — the CLI sends prefs and exits immediately.
    // When the CLI disconnects, daemon cleanup calls profileDirFor() before the new
    // profile is committed to knownProfiles — a race window that only exists in a
    // freshly-restarted daemon. A daemon that has been running since app launch is
    // fully warm and doesn't hit this race.
    //
    // Strategy:
    // - If daemon is alive: just logout (clears current profile + resets
    //   blockEngineUpdates if a previous attempt hit it) then tailscale up.
    // - If daemon is dead: full restart (kill + wipe + start). Accept the race
    //   risk — this is the edge case (crash), not the normal retry path.
    if daemon_can_respond(&app, 3).await {
        // Daemon is alive — logout clears the profile and unblocks the engine.
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            run_ts(&app, &["logout"]),
        ).await;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    } else {
        // Daemon is dead — full restart.
        app.state::<DaemonHandle>().kill();
        let state_dir = data_dir(&app);
        if state_dir.exists() {
            let _ = std::fs::remove_dir_all(&state_dir);
        }
        let _ = std::fs::create_dir_all(&state_dir);
        start_daemon(&app);
        if !daemon_can_respond(&app, 15).await {
            let log = read_daemon_log(&app);
            let detail = if log.trim().is_empty() {
                "No daemon output captured.".to_string()
            } else {
                let start = log.len().saturating_sub(800);
                format!("Daemon log:\n{}", &log[start..])
            };
            return Err(format!("Tailscale daemon failed to start.\n{detail}"));
        }
    }

    let out = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        tokio::process::Command::new(tailscale_bin(&app))
            .args([
                "--socket", &socket,
                "up",
                "--login-server",  &login_server,
                "--authkey",       &authkey,
                "--hostname",      &hostname,
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
