mod caddy;
mod config;
mod git_sync;
mod hosts_file;
mod models;
mod service;

use models::{CaddyStatus, Config, GitStatus, Host, ServiceStatus};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Manager,
};

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

/// Add a new host if the id is empty, otherwise update the existing host.
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

/// Preview the contents of the hosts file that will be written.
#[tauri::command]
fn preview_hosts() -> Result<String, String> {
    let cfg = config::load_config()?;
    hosts_file::render_hosts(&cfg)
}

/// Preview the Caddyfile that will be generated.
#[tauri::command]
fn preview_caddyfile() -> Result<String, String> {
    let cfg = config::load_config()?;
    Ok(caddy::generate_caddyfile(&cfg))
}

/// Write the hosts file (requesting admin privileges) and reload/start caddy.
#[tauri::command]
fn apply_all() -> Result<(), String> {
    let cfg = config::load_config()?;
    hosts_file::apply(&cfg)?;
    caddy::apply(&cfg)?;
    Ok(())
}

/// Only update the hosts file.
#[tauri::command]
fn apply_hosts() -> Result<(), String> {
    let cfg = config::load_config()?;
    hosts_file::apply(&cfg)
}

/// Open the hosts file (/etc/hosts) in the default editor.
#[tauri::command]
fn open_hosts_file() -> Result<(), String> {
    hosts_file::open_in_editor()
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

/// Install Caddy's local CA into the system trust store (for trusted HTTPS).
#[tauri::command]
fn caddy_trust() -> Result<(), String> {
    caddy::trust()
}

// ---------- Background service (launchd) ----------

#[tauri::command]
fn service_status() -> ServiceStatus {
    service::status()
}

#[tauri::command]
fn service_install() -> Result<ServiceStatus, String> {
    service::install()?;
    Ok(service::status())
}

#[tauri::command]
fn service_uninstall() -> Result<ServiceStatus, String> {
    service::uninstall()?;
    Ok(service::status())
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

/// Show and focus the main window.
fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Create the system tray icon with a Show / Quit menu.
fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_i = MenuItem::with_id(app, "show", "Open Hostman", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_i, &sep, &quit_i])?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Hostman")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            setup_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window hides it to the tray instead of quitting (the app keeps running in the background).
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
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
            open_hosts_file,
            caddy_status,
            caddy_start,
            caddy_stop,
            caddy_reload,
            caddy_trust,
            service_status,
            service_install,
            service_uninstall,
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
