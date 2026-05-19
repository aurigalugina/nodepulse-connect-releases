mod commands;
mod tray;

use commands::{
    storage::{clear_auth_token, read_config, write_config},
    tailscale::{detect_tailscale, tailscale_down, tailscale_status, tailscale_up},
};
use tray::{set_tray_connected, setup_tray};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            setup_tray(&app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Tailscale
            detect_tailscale,
            tailscale_status,
            tailscale_up,
            tailscale_down,
            // Storage
            read_config,
            write_config,
            clear_auth_token,
            // Tray
            set_tray_connected,
        ])
        .run(tauri::generate_context!())
        .expect("error while running NodePulse Connect");
}
