use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct AppConfig {
    pub nodepulse_url: Option<String>,
    pub username: Option<String>,
    pub auth_token: Option<String>,
    pub last_cluster_id: Option<String>,
    pub device_name: Option<String>,
}

fn config_path() -> Result<PathBuf, String> {
    let appdata = std::env::var("APPDATA")
        .map_err(|_| "APPDATA environment variable not set".to_string())?;
    let dir = PathBuf::from(appdata).join("NodePulse Connect");
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create config directory: {e}"))?;
    Ok(dir.join("config.json"))
}

/// Read persisted config from %APPDATA%\NodePulse Connect\config.json.
/// Returns default (empty) config if file does not exist yet.
#[tauri::command]
pub fn read_config() -> Result<AppConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let data = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read config: {e}"))?;
    serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse config: {e}"))
}

/// Write config to %APPDATA%\NodePulse Connect\config.json atomically.
#[tauri::command]
pub fn write_config(config: AppConfig) -> Result<(), String> {
    let path = config_path()?;
    let tmp_path = path.with_extension("tmp");

    let data = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;

    fs::write(&tmp_path, data)
        .map_err(|e| format!("Failed to write config: {e}"))?;

    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to finalize config write: {e}"))?;

    Ok(())
}

/// Clear the stored auth token (on logout), preserving other fields.
#[tauri::command]
pub fn clear_auth_token() -> Result<(), String> {
    let mut config = read_config()?;
    config.auth_token = None;
    write_config(config)
}
