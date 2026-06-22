use crate::config;
use crate::models::{CaddyStatus, Config, PathRoute};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const ADMIN_ADDR: &str = "localhost:2019";

/// Locate the caddy binary: env var -> sidecar next to the executable -> PATH.
pub fn find_caddy() -> Option<PathBuf> {
    // 0. Override via env var (convenient for development).
    if let Ok(p) = std::env::var("HOSTMAN_CADDY") {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }
    // 1. Sidecar next to the app executable (Tauri bundle).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in caddy_names() {
                let candidate = dir.join(name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }
    // 2. PATH.
    let probe = if cfg!(windows) { "where" } else { "which" };
    if let Ok(out) = Command::new(probe).arg("caddy").output() {
        if out.status.success() {
            let line = String::from_utf8_lossy(&out.stdout);
            if let Some(first) = line.lines().next() {
                let p = PathBuf::from(first.trim());
                if !p.as_os_str().is_empty() {
                    return Some(p);
                }
            }
        }
    }
    None
}

fn caddy_names() -> Vec<&'static str> {
    if cfg!(windows) {
        vec!["caddy.exe"]
    } else {
        vec!["caddy"]
    }
}

/// Normalize a path into a Caddy matcher: ensure a leading "/" and a trailing "*".
/// E.g. "/admin" -> "/admin*", "admin" -> "/admin*".
fn path_matcher(path: &str) -> String {
    let p = path.trim();
    let p = if p.starts_with('/') {
        p.to_string()
    } else {
        format!("/{p}")
    };
    if p.ends_with('*') {
        p
    } else {
        format!("{p}*")
    }
}

/// Render the body of a site block (the directives inside the braces).
fn render_site_body(default_target: &str, paths: &[PathRoute]) -> String {
    let active: Vec<&PathRoute> = paths
        .iter()
        .filter(|p| !p.path.trim().is_empty() && !p.target.trim().is_empty())
        .collect();

    // No specific paths -> proxy straight to the default target.
    if active.is_empty() {
        return format!("\treverse_proxy {default_target}\n");
    }

    let mut body = String::new();
    for p in &active {
        let matcher = path_matcher(&p.path);
        // handle_path strips the prefix before proxying; handle keeps the path intact.
        let directive = if p.strip_prefix { "handle_path" } else { "handle" };
        body.push_str(&format!(
            "\t{directive} {matcher} {{\n\t\treverse_proxy {}\n\t}}\n",
            p.target
        ));
    }
    // Catch-all for any remaining paths -> default target.
    body.push_str(&format!(
        "\thandle {{\n\t\treverse_proxy {default_target}\n\t}}\n"
    ));
    body
}

/// Generate the Caddyfile contents from the config.
pub fn generate_caddyfile(cfg: &Config) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("\tadmin {ADMIN_ADDR}\n"));
    out.push_str("}\n\n");

    for h in &cfg.hosts {
        if !h.enabled {
            continue;
        }
        // For internal TLDs (.test, .localhost) Caddy automatically uses its internal CA for HTTPS.
        let site = if h.https {
            h.domain.clone()
        } else {
            format!("http://{}", h.domain)
        };
        let body = render_site_body(&h.target, &h.paths);
        out.push_str(&format!("{site} {{\n{body}}}\n\n"));
    }
    out
}

/// Write the Caddyfile to disk.
pub fn write_caddyfile(cfg: &Config) -> Result<PathBuf, String> {
    config::ensure_dir()?;
    let path = config::caddyfile_path()?;
    fs::write(&path, generate_caddyfile(cfg)).map_err(|e| format!("Failed to write Caddyfile: {e}"))?;
    Ok(path)
}

/// Whether Caddy is currently running (checks the admin API).
pub fn is_running() -> bool {
    let caddy = match find_caddy() {
        Some(c) => c,
        None => return false,
    };
    // `caddy adapt` doesn't need a server; using `caddy version` + probing the admin API via curl isn't available.
    // Simple approach: trying a reload with an empty config errors if it isn't running -> using a pidfile instead isn't cross-platform.
    // For now: there's no built-in `caddy ...` command to query the admin API -> use a TcpStream.
    use std::net::TcpStream;
    let _ = caddy;
    TcpStream::connect(ADMIN_ADDR).is_ok()
}

pub fn status() -> CaddyStatus {
    CaddyStatus {
        running: is_running(),
        binary: find_caddy().map(|p| p.display().to_string()),
    }
}

fn caddy_cmd() -> Result<Command, String> {
    let bin = find_caddy().ok_or_else(|| {
        "caddy binary not found. Install caddy or place a sidecar next to the app.".to_string()
    })?;
    Ok(Command::new(bin))
}

/// Start caddy (in the background). Binding 80/443 requires root on macOS/Linux,
/// so on those OSes it will prompt for admin privileges.
pub fn start(cfg: &Config) -> Result<(), String> {
    let path = write_caddyfile(cfg)?;
    let bin = find_caddy().ok_or_else(|| {
        "caddy binary not found. Install caddy or place a sidecar next to the app.".to_string()
    })?;

    #[cfg(target_os = "macos")]
    {
        // Run `caddy start` with admin privileges via the system dialog.
        let script = format!(
            "do shell script \"'{}' start --config '{}'\" with administrator privileges",
            bin.display(),
            path.display()
        );
        let status = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status()
            .map_err(|e| format!("Failed to run osascript: {e}"))?;
        if !status.success() {
            return Err("caddy start failed (admin privileges denied?)".into());
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let status = Command::new("pkexec")
            .arg(bin.as_os_str())
            .arg("start")
            .arg("--config")
            .arg(&path)
            .status()
            .map_err(|e| format!("Failed to run pkexec: {e}"))?;
        if !status.success() {
            return Err("caddy start failed (admin privileges denied?)".into());
        }
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, binding 80/443 usually doesn't require admin (or uses a URL ACL).
        let status = Command::new(&bin)
            .arg("start")
            .arg("--config")
            .arg(&path)
            .status()
            .map_err(|e| format!("caddy start error: {e}"))?;
        if !status.success() {
            return Err("caddy start failed".into());
        }
        Ok(())
    }
}

/// Reload the config with no downtime (via the admin API).
pub fn reload(cfg: &Config) -> Result<(), String> {
    let path = write_caddyfile(cfg)?;
    let status = caddy_cmd()?
        .arg("reload")
        .arg("--config")
        .arg(&path)
        .status()
        .map_err(|e| format!("caddy reload error: {e}"))?;
    if !status.success() {
        return Err("caddy reload failed".into());
    }
    Ok(())
}

pub fn stop() -> Result<(), String> {
    let status = caddy_cmd()?
        .arg("stop")
        .status()
        .map_err(|e| format!("caddy stop error: {e}"))?;
    if !status.success() {
        return Err("caddy stop failed".into());
    }
    Ok(())
}

/// Apply: reload if running, otherwise start.
pub fn apply(cfg: &Config) -> Result<(), String> {
    if is_running() {
        reload(cfg)
    } else {
        start(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Host;

    fn host(domain: &str, target: &str, paths: Vec<PathRoute>) -> Host {
        Host {
            id: "1".into(),
            name: "x".into(),
            domain: domain.into(),
            target: target.into(),
            https: true,
            enabled: true,
            paths,
        }
    }

    #[test]
    fn no_paths_renders_simple_reverse_proxy() {
        let cfg = Config {
            default_tld: "test".into(),
            hosts: vec![host("a.test", "localhost:3000", vec![])],
        };
        let out = generate_caddyfile(&cfg);
        assert!(out.contains("a.test {\n\treverse_proxy localhost:3000\n}"));
        assert!(!out.contains("handle"));
    }

    #[test]
    fn path_route_renders_handle_blocks_with_catch_all() {
        let routes = vec![PathRoute {
            path: "/admin".into(),
            target: "localhost:4000".into(),
            strip_prefix: false,
        }];
        let cfg = Config {
            default_tld: "test".into(),
            hosts: vec![host("a.test", "localhost:3000", routes)],
        };
        let out = generate_caddyfile(&cfg);
        assert!(out.contains("handle /admin* {"));
        assert!(out.contains("reverse_proxy localhost:4000"));
        // Catch-all keeps the default target.
        assert!(out.contains("handle {\n\t\treverse_proxy localhost:3000"));
    }

    #[test]
    fn strip_prefix_uses_handle_path() {
        let routes = vec![PathRoute {
            path: "admin".into(), // no leading "/" -> added automatically
            target: "localhost:4000".into(),
            strip_prefix: true,
        }];
        let cfg = Config {
            default_tld: "test".into(),
            hosts: vec![host("a.test", "localhost:3000", routes)],
        };
        let out = generate_caddyfile(&cfg);
        assert!(out.contains("handle_path /admin* {"));
    }

    #[test]
    fn empty_path_entries_are_ignored() {
        let routes = vec![PathRoute {
            path: "  ".into(),
            target: "".into(),
            strip_prefix: false,
        }];
        let cfg = Config {
            default_tld: "test".into(),
            hosts: vec![host("a.test", "localhost:3000", routes)],
        };
        let out = generate_caddyfile(&cfg);
        assert!(out.contains("a.test {\n\treverse_proxy localhost:3000\n}"));
        assert!(!out.contains("handle"));
    }
}

/// Install Caddy's local CA into the system trust store (`caddy trust`).
/// Only needs to run once; requires admin privileges to write to the system trust store.
pub fn trust() -> Result<(), String> {
    let bin = find_caddy().ok_or_else(|| {
        "caddy binary not found. Install caddy or place a sidecar next to the app.".to_string()
    })?;

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "do shell script \"'{}' trust\" with administrator privileges",
            bin.display()
        );
        let status = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status()
            .map_err(|e| format!("Failed to run osascript: {e}"))?;
        if !status.success() {
            return Err("caddy trust failed (admin privileges denied?)".into());
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let status = Command::new("pkexec")
            .arg(bin.as_os_str())
            .arg("trust")
            .status()
            .map_err(|e| format!("Failed to run pkexec: {e}"))?;
        if !status.success() {
            return Err("caddy trust failed (admin privileges denied?)".into());
        }
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        let status = Command::new(&bin)
            .arg("trust")
            .status()
            .map_err(|e| format!("caddy trust error: {e}"))?;
        if !status.success() {
            return Err("caddy trust failed".into());
        }
        Ok(())
    }
}
