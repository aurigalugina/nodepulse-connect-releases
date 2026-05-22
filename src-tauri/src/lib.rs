mod commands;
mod tray;

/// Tell Windows shell to flush icon cache so desktop/taskbar shortcuts show current icon.
/// Uses shell32's SHChangeNotify — no extra crate needed.
#[cfg(target_os = "windows")]
fn refresh_icon_cache() {
    #[link(name = "shell32")]
    extern "system" {
        fn SHChangeNotify(
            wEventId: i32,
            uFlags: u32,
            dwItem1: *const std::ffi::c_void,
            dwItem2: *const std::ffi::c_void,
        );
    }
    unsafe {
        // SHCNE_ASSOCCHANGED (0x08000000) + SHCNF_IDLIST (0) — flushes all icon lookups
        SHChangeNotify(0x0800_0000i32, 0, std::ptr::null(), std::ptr::null());
    }
}

use tauri::Manager;

use commands::{
    identity::get_device_identity,
    storage::{clear_auth_token, read_config, write_config},
    tailscale::{ensure_tailscale, get_daemon_log, launch_daemon, stop_daemon, tailscale_down, tailscale_is_ready, tailscale_status, tailscale_up, DaemonHandle},
};
use tray::{set_tray_connected, setup_tray};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(DaemonHandle::new())
        .setup(|app| {
            setup_tray(&app.handle())?;
            // Refresh Windows icon cache so desktop/taskbar shortcuts show the current icon.
            #[cfg(target_os = "windows")]
            refresh_icon_cache();
            // NOTE: start_daemon is NOT called here — it is called from the frontend
            // after the startup update check completes (via launch_daemon command).
            // This ensures tailscaled.exe is never running when an update installs.
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Tailscale
            tailscale_is_ready,
            ensure_tailscale,
            tailscale_status,
            tailscale_up,
            tailscale_down,
            get_daemon_log,
            stop_daemon,
            launch_daemon,
            // Storage
            read_config,
            write_config,
            clear_auth_token,
            // Tray
            set_tray_connected,
            // Identity
            get_device_identity,
        ])
        .build(tauri::generate_context!())
        .expect("error building NodePulse Connect")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                // Kill the isolated tailscaled daemon on app exit.
                app_handle.state::<DaemonHandle>().kill();
            }
        });
}
