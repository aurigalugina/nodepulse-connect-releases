mod commands;
mod tray;

use tauri::Manager;

use commands::{
    identity::get_device_identity,
    storage::{clear_auth_token, read_config, write_config},
    tailscale::{ensure_tailscale, get_daemon_log, tailscale_down, tailscale_is_ready, tailscale_status, tailscale_up, DaemonHandle},
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
            // Start tailscaled if already downloaded; noop on first launch (binary absent).
            commands::tailscale::start_daemon(&app.handle());
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
