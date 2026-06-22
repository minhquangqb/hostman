use crate::models::Config;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Root directory: ~/.hostman
pub fn base_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "Home directory not found".to_string())?;
    Ok(home.join(".hostman"))
}

/// Config directory synced via git: ~/.hostman/config
pub fn config_dir() -> Result<PathBuf, String> {
    Ok(base_dir()?.join("config"))
}

/// Main config file: ~/.hostman/config/hosts.json
pub fn config_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("hosts.json"))
}

/// Generated Caddyfile: ~/.hostman/config/Caddyfile
pub fn caddyfile_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("Caddyfile"))
}

pub fn ensure_dir() -> Result<(), String> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create {}: {e}", dir.display()))
}

pub fn load_config() -> Result<Config, String> {
    ensure_dir()?;
    let path = config_path()?;
    if !path.exists() {
        let cfg = Config::default();
        save_config(&cfg)?;
        return Ok(cfg);
    }
    let data = fs::read_to_string(&path).map_err(|e| format!("Failed to read config: {e}"))?;
    serde_json::from_str(&data).map_err(|e| format!("Failed to parse config: {e}"))
}

pub fn save_config(cfg: &Config) -> Result<(), String> {
    ensure_dir()?;
    let data = serde_json::to_string_pretty(cfg).map_err(|e| format!("Failed to serialize: {e}"))?;
    fs::write(config_path()?, data).map_err(|e| format!("Failed to write config: {e}"))
}

/// Generate a unique id from the current time (nanos).
pub fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("h{nanos}")
}
