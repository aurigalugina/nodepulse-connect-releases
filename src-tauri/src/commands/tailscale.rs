use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use serde::{Deserialize, Serialize};

// ── Daemon state ───────────────────────────────────────────────────────────────

pub struct DaemonHandle {
    // Windows: always None — daemon runs as Windows Service.
    // Linux/macOS: child process handle.
    pub child: Mutex<Option<std::process::Child>>,
}

impl DaemonHandle {
    pub fn new() -> Self { Self { child: Mutex::new(None) } }

    pub fn kill(&self) {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("sc")
                .args(["stop", "NodePulseConnectDaemon"])
                .output();
            // Give SCM time to stop the service and release file/socket locks.
            std::thread::sleep(std::time::Duration::from_millis(2500));
        }
        #[cfg(not(target_os = "windows"))]
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
    // Windows: service runs as LocalSystem — statedir lives in ProgramData so
    // both the service (SYSTEM) and the user app can access it.
    #[cfg(target_os = "windows")]
    {
        let _ = app;
        return PathBuf::from(
            std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".to_string())
        ).join("NodePulse Connect").join("tailscale-state");
    }
    #[cfg(not(target_os = "windows"))]
    app.path().app_data_dir().expect("app data dir").join("tailscale-state")
}

pub fn socket_path(app: &tauri::AppHandle) -> String {
    #[cfg(target_os = "windows")]
    return r"\\.\pipe\NodePulseConnect-tailscaled".to_string();

    #[cfg(not(target_os = "windows"))]
    data_dir(app).join("tailscaled.sock").to_str().unwrap().to_string()
}

// ── Socket readiness poll ──────────────────────────────────────────────────────

/// Block until the daemon socket/pipe is ready.
/// Windows: service may take several seconds to start — allow up to ~18s.
/// Linux/macOS: child process is faster — allow up to ~5s.
fn wait_for_socket(socket: &str) {
    #[cfg(target_os = "windows")]
    let (probes, interval_ms) = (60u32, 300u64);
    #[cfg(not(target_os = "windows"))]
    let (probes, interval_ms) = (25u32, 200u64);

    for _ in 0..probes {
        if socket_is_ready(socket) { return; }
        std::thread::sleep(std::time::Duration::from_millis(interval_ms));
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

/// Return up to the first 1 KB + last 2 KB of the daemon log so we see both
/// startup context and the most recent entries.
fn log_snippet(log: &str) -> String {
    const HEAD: usize = 1024;
    const TAIL: usize = 2048;
    if log.len() <= HEAD + TAIL {
        return log.to_string();
    }
    let head = &log[..HEAD];
    let tail_start = log.len().saturating_sub(TAIL);
    let tail = &log[tail_start..];
    format!("{head}\n…[truncated]…\n{tail}")
}

// ── Daemon lifecycle ───────────────────────────────────────────────────────────

/// Windows: start the NodePulseConnectDaemon Windows service (installed by NSIS).
/// The service runs as LocalSystem and uses WinTun for kernel-mode routing,
/// giving the OS proper routes to 100.64.0.0/10 so browsers/SSH can reach mesh IPs.
///
/// Linux/macOS: spawn tailscaled as a child process with userspace networking.
pub fn start_daemon(app: &tauri::AppHandle) {
    #[cfg(target_os = "windows")]
    {
        // Check if service is already running.
        let running = std::process::Command::new("sc")
            .args(["query", "NodePulseConnectDaemon"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("RUNNING"))
            .unwrap_or(false);
        if running { return; }

        let _ = std::process::Command::new("sc")
            .args(["start", "NodePulseConnectDaemon"])
            .output();

        let socket = socket_path(app);
        wait_for_socket(&socket);
    }

    #[cfg(not(target_os = "windows"))]
    {
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

        // Remove stale socket file before spawning.
        let sock = PathBuf::from(&socket);
        if sock.exists() { let _ = std::fs::remove_file(&sock); }

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

        // Use file-based state. With mem: the profile dir cleanup races on CLI exit.
        // With a file-based state the profile is written to disk before the CLI exits.
        let state_file = state_dir.join("tailscale.state");
        let state_file_str = state_file.to_str().unwrap_or("").to_string();

        match std::process::Command::new(&bin)
            .args([
                "--tun=userspace-networking",
                "--socket", &socket,
                "--statedir", state_dir.to_str().unwrap_or(""),
                "--state", &state_file_str,
            ])
            // Skip WinHTTP proxy detection on macOS/Linux (no-op but harmless).
            .env("NO_PROXY", "*")
            .env("no_proxy", "*")
            .stdout(stdout_s)
            .stderr(stderr_s)
            .spawn()
        {
            Ok(child) => {
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
}

#[tauri::command]
pub fn get_daemon_log(app: tauri::AppHandle) -> String {
    read_daemon_log(&app)
}

// Kill the daemon process / stop the service — called before update install so
// the NSIS extractor can overwrite tailscaled.exe.
#[tauri::command]
pub fn stop_daemon(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let _ = app;
        let _ = std::process::Command::new("sc")
            .args(["stop", "NodePulseConnectDaemon"])
            .output();
        std::thread::sleep(std::time::Duration::from_millis(3000));
    }
    #[cfg(not(target_os = "windows"))]
    app.state::<DaemonHandle>().kill();
    Ok(())
}

// Start the daemon / service — called from frontend AFTER the startup update
// check completes, so the binary is never locked when an update installs.
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
/// Each probe has a 3 s hard timeout so a slow daemon cannot inflate the total wait.
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
            _ => {}
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

    // Stop daemon/service + wipe statedir + restart fresh before every connect attempt.
    //
    // WHY WIPE: If tailscale.state exists from a previous session with WantRunning=true,
    // the daemon reads it on startup and may enter a blockEngineUpdates state.
    // With a clean statedir the daemon starts in pure NoState with no auto-init.
    //
    // Windows: kill() stops the service via SCM; statedir is in ProgramData and
    // the user process has write access (granted by NSIS icacls during installation).
    let _ = app.emit("connect-debug", "[rust] killing daemon…");
    app.state::<DaemonHandle>().kill();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let state_dir = data_dir(&app);
    let _ = std::fs::remove_dir_all(&state_dir);
    let _ = std::fs::create_dir_all(&state_dir);

    let _ = app.emit("connect-debug", "[rust] starting fresh daemon…");
    start_daemon(&app);

    let _ = app.emit("connect-debug", "[rust] waiting for IPC socket (15 probes)…");
    if !daemon_can_respond(&app, 15).await {
        let _ = app.emit("connect-debug", "[rust] daemon failed to start");
        let log = read_daemon_log(&app);
        let detail = if log.trim().is_empty() { "No daemon output captured.".to_string() }
            else { format!("Daemon log:\n{}", log_snippet(&log)) };
        return Err(format!("Tailscale daemon failed to start.\n{detail}"));
    }

    // With the patched tailscaled binary (serverMode always true + Patch 6 profile fix),
    // b.Start() no longer resets WantRunning when an IPN client disconnects and
    // the profile switch to empty is prevented. doLogin runs to completion.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Poll until BackendState exits NoState.
    for i in 0..20u8 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let probe = run_ts(&app, &["status", "--json"]).await;
        let backend_state = probe.ok()
            .and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
            .and_then(|v| v["BackendState"].as_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string());
        let _ = app.emit("connect-debug", format!("[rust] init poll {}/{}: BackendState={backend_state}", i + 1, 20));
        if backend_state != "NoState" && backend_state != "unknown" {
            let _ = app.emit("connect-debug", "[rust] profile manager ready");
            break;
        }
    }

    let _ = app.emit("connect-debug",
        format!("[rust] tailscale up --login-server {} --hostname {} --timeout=60s", login_server, hostname));

    let up_result = tokio::time::timeout(
        std::time::Duration::from_secs(75),
        tokio::process::Command::new(tailscale_bin(&app))
            .args([
                "--socket",        &socket,
                "up",
                "--login-server",  &login_server,
                "--authkey",       &authkey,
                "--hostname",      &hostname,
                "--timeout",       "60s",
            ])
            .output(),
    ).await;

    let out = match up_result {
        Err(_) => {
            let _ = app.emit("connect-debug", "[rust] tailscale up timed out after 75s");
            let log = read_daemon_log(&app);
            return Err(format!("tailscale up prefs send timed out.\nDaemon log:\n{}", log_snippet(&log)));
        }
        Ok(Err(e)) => {
            let _ = app.emit("connect-debug", format!("[rust] tailscale up spawn error: {e}"));
            return Err(format!("tailscale up: {e}"));
        }
        Ok(Ok(o)) => o,
    };

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let _ = app.emit("connect-debug",
        format!("[rust] tailscale up exit={} stdout={:?} stderr={:?}",
            out.status.code().unwrap_or(-1),
            stdout.trim(), stderr.trim()));

    if !out.status.success() {
        let msg = if !stderr.is_empty() { stderr } else { stdout };
        return Err(msg);
    }

    // tailscale up sent prefs — poll until daemon reaches Running state (up to 75s).
    let _ = app.emit("connect-debug", "[rust] prefs sent — polling for Running state (75s)…");
    for i in 1u32..=25 {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let probe = run_ts(&app, &["status", "--json"]).await;
        let (state, online) = probe.ok()
            .and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
            .map(|v| (
                v["BackendState"].as_str().unwrap_or("unknown").to_string(),
                v["Self"]["Online"].as_bool().unwrap_or(false),
            ))
            .unwrap_or_else(|| ("unknown".to_string(), false));
        let _ = app.emit("connect-debug",
            format!("[rust] +{}s BackendState={state} online={online}", i * 3));
        if state == "Running" && online {
            let _ = app.emit("connect-debug", "[rust] connected!");
            return Ok("Connected".to_string());
        }
        if state == "Stopped" {
            let log = read_daemon_log(&app);
            let snip = log_snippet(&log);
            let _ = app.emit("connect-debug", format!("[rust] daemon stopped early: {snip}"));
            return Err(format!(
                "Mesh connection failed (daemon: {state}).\nDaemon log:\n{snip}"
            ));
        }
    }

    let log = read_daemon_log(&app);
    let snip = log_snippet(&log);
    Err(format!(
        "Mesh connection timed out after 75s (daemon: NoState). DERP relay may be unreachable.\nDaemon log:\n{snip}"
    ))
}

#[tauri::command]
pub async fn tailscale_down(app: tauri::AppHandle) -> Result<(), String> {
    // With serverMode=true (patched binary), tailscale down sets WantRunning=false
    // and the daemon stops routing. Next connection attempt kills+restarts anyway.
    let _ = run_ts(&app, &["down"]).await;
    Ok(())
}
