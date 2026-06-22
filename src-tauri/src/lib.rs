mod caddy;
mod config;
mod git_sync;
mod hosts_file;
mod models;

use models::{CaddyStatus, Config, GitStatus, Host};

// ---------- Config / Host CRUD ----------

#[tauri::command]
fn get_config() -> Result<Config, String> {
    config::load_config()
}

#[tauri::command]
fn default_tld() -> Result<String, String> {
    Ok(config::load_config()?.default_tld)
}

#[tauri::command]
fn set_default_tld(tld: String) -> Result<Config, String> {
    let mut cfg = config::load_config()?;
    cfg.default_tld = tld;
    config::save_config(&cfg)?;
    Ok(cfg)
}

/// Them moi neu id rong, hoac cap nhat host co san.
#[tauri::command]
fn save_host(mut host: Host) -> Result<Config, String> {
    let mut cfg = config::load_config()?;
    if host.id.trim().is_empty() {
        host.id = config::new_id();
        cfg.hosts.push(host);
    } else {
        match cfg.hosts.iter_mut().find(|h| h.id == host.id) {
            Some(existing) => *existing = host,
            None => cfg.hosts.push(host),
        }
    }
    config::save_config(&cfg)?;
    Ok(cfg)
}

#[tauri::command]
fn delete_host(id: String) -> Result<Config, String> {
    let mut cfg = config::load_config()?;
    cfg.hosts.retain(|h| h.id != id);
    config::save_config(&cfg)?;
    Ok(cfg)
}

#[tauri::command]
fn toggle_host(id: String, enabled: bool) -> Result<Config, String> {
    let mut cfg = config::load_config()?;
    if let Some(h) = cfg.hosts.iter_mut().find(|h| h.id == id) {
        h.enabled = enabled;
    }
    config::save_config(&cfg)?;
    Ok(cfg)
}

// ---------- Apply (hosts file + caddy) ----------

/// Preview noi dung hosts file se duoc ghi.
#[tauri::command]
fn preview_hosts() -> Result<String, String> {
    let cfg = config::load_config()?;
    hosts_file::render_hosts(&cfg)
}

/// Preview Caddyfile se duoc sinh.
#[tauri::command]
fn preview_caddyfile() -> Result<String, String> {
    let cfg = config::load_config()?;
    Ok(caddy::generate_caddyfile(&cfg))
}

/// Ghi hosts file (xin quyen admin) va reload/start caddy.
#[tauri::command]
fn apply_all() -> Result<(), String> {
    let cfg = config::load_config()?;
    hosts_file::apply(&cfg)?;
    caddy::apply(&cfg)?;
    Ok(())
}

/// Chi cap nhat hosts file.
#[tauri::command]
fn apply_hosts() -> Result<(), String> {
    let cfg = config::load_config()?;
    hosts_file::apply(&cfg)
}

// ---------- Caddy control ----------

#[tauri::command]
fn caddy_status() -> CaddyStatus {
    caddy::status()
}

#[tauri::command]
fn caddy_start() -> Result<(), String> {
    let cfg = config::load_config()?;
    caddy::start(&cfg)
}

#[tauri::command]
fn caddy_stop() -> Result<(), String> {
    caddy::stop()
}

#[tauri::command]
fn caddy_reload() -> Result<(), String> {
    let cfg = config::load_config()?;
    caddy::reload(&cfg)
}

// ---------- Git sync ----------

#[tauri::command]
fn git_status() -> GitStatus {
    git_sync::status()
}

#[tauri::command]
fn git_init() -> Result<GitStatus, String> {
    git_sync::init()?;
    Ok(git_sync::status())
}

#[tauri::command]
fn git_set_remote(url: String) -> Result<GitStatus, String> {
    git_sync::set_remote(&url)?;
    Ok(git_sync::status())
}

#[tauri::command]
fn git_commit(message: String) -> Result<GitStatus, String> {
    git_sync::commit(&message)?;
    Ok(git_sync::status())
}

#[tauri::command]
fn git_pull() -> Result<String, String> {
    git_sync::pull()
}

#[tauri::command]
fn git_push() -> Result<String, String> {
    git_sync::push()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            default_tld,
            set_default_tld,
            save_host,
            delete_host,
            toggle_host,
            preview_hosts,
            preview_caddyfile,
            apply_all,
            apply_hosts,
            caddy_status,
            caddy_start,
            caddy_stop,
            caddy_reload,
            git_status,
            git_init,
            git_set_remote,
            git_commit,
            git_pull,
            git_push,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
