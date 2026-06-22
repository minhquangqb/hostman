use crate::config;
use crate::models::{CaddyStatus, Config, PathRoute};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const ADMIN_ADDR: &str = "localhost:2019";

/// Tim caddy binary: env var -> sidecar canh executable -> PATH.
pub fn find_caddy() -> Option<PathBuf> {
    // 0. Override qua env var (tien cho dev).
    if let Ok(p) = std::env::var("HOSTMAN_CADDY") {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }
    // 1. Sidecar canh executable cua app (Tauri bundle).
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

/// Chuan hoa path thanh matcher cua Caddy: dam bao co dau "/" o dau va "*" o cuoi.
/// Vd "/admin" -> "/admin*", "admin" -> "/admin*".
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

/// Sinh body cua mot site block (cac directive ben trong dau ngoac).
fn render_site_body(default_target: &str, paths: &[PathRoute]) -> String {
    let active: Vec<&PathRoute> = paths
        .iter()
        .filter(|p| !p.path.trim().is_empty() && !p.target.trim().is_empty())
        .collect();

    // Khong co path rieng -> proxy thang toi target mac dinh.
    if active.is_empty() {
        return format!("\treverse_proxy {default_target}\n");
    }

    let mut body = String::new();
    for p in &active {
        let matcher = path_matcher(&p.path);
        // handle_path bo tien to truoc khi proxy; handle giu nguyen path.
        let directive = if p.strip_prefix { "handle_path" } else { "handle" };
        body.push_str(&format!(
            "\t{directive} {matcher} {{\n\t\treverse_proxy {}\n\t}}\n",
            p.target
        ));
    }
    // Catch-all cho moi path con lai -> target mac dinh.
    body.push_str(&format!(
        "\thandle {{\n\t\treverse_proxy {default_target}\n\t}}\n"
    ));
    body
}

/// Sinh noi dung Caddyfile tu config.
pub fn generate_caddyfile(cfg: &Config) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("\tadmin {ADMIN_ADDR}\n"));
    out.push_str("}\n\n");

    for h in &cfg.hosts {
        if !h.enabled {
            continue;
        }
        // Voi TLD noi bo (.test, .localhost) Caddy tu dung internal CA cho HTTPS.
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

/// Ghi Caddyfile ra dia.
pub fn write_caddyfile(cfg: &Config) -> Result<PathBuf, String> {
    config::ensure_dir()?;
    let path = config::caddyfile_path()?;
    fs::write(&path, generate_caddyfile(cfg)).map_err(|e| format!("Ghi Caddyfile loi: {e}"))?;
    Ok(path)
}

/// Caddy co dang chay khong (kiem tra admin API).
pub fn is_running() -> bool {
    let caddy = match find_caddy() {
        Some(c) => c,
        None => return false,
    };
    // `caddy adapt` khong can server; dung `caddy version` + thu admin API qua curl khong kha dung.
    // Cach don gian: thu reload voi config rong se loi neu chua chay -> thay vao do dung pidfile khong cross-platform.
    // Tam thoi: hoi admin API qua chinh caddy bang lenh `caddy ... ` khong co san -> dung TcpStream.
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
        "Khong tim thay caddy binary. Cai caddy hoac dat sidecar canh app.".to_string()
    })?;
    Ok(Command::new(bin))
}

/// Khoi dong caddy (background). Bind 80/443 can quyen root tren macOS/Linux
/// nen tren cac OS do se xin quyen admin.
pub fn start(cfg: &Config) -> Result<(), String> {
    let path = write_caddyfile(cfg)?;
    let bin = find_caddy().ok_or_else(|| {
        "Khong tim thay caddy binary. Cai caddy hoac dat sidecar canh app.".to_string()
    })?;

    #[cfg(target_os = "macos")]
    {
        // Chay `caddy start` voi quyen admin qua dialog he thong.
        let script = format!(
            "do shell script \"'{}' start --config '{}'\" with administrator privileges",
            bin.display(),
            path.display()
        );
        let status = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status()
            .map_err(|e| format!("Chay osascript loi: {e}"))?;
        if !status.success() {
            return Err("caddy start that bai (cap quyen admin bi tu choi?)".into());
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
            .map_err(|e| format!("Chay pkexec loi: {e}"))?;
        if !status.success() {
            return Err("caddy start that bai (cap quyen admin bi tu choi?)".into());
        }
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        // Windows thuong bind 80/443 khong can admin (hoac dung URL ACL).
        let status = Command::new(&bin)
            .arg("start")
            .arg("--config")
            .arg(&path)
            .status()
            .map_err(|e| format!("caddy start loi: {e}"))?;
        if !status.success() {
            return Err("caddy start that bai".into());
        }
        Ok(())
    }
}

/// Reload config khong downtime (qua admin API).
pub fn reload(cfg: &Config) -> Result<(), String> {
    let path = write_caddyfile(cfg)?;
    let status = caddy_cmd()?
        .arg("reload")
        .arg("--config")
        .arg(&path)
        .status()
        .map_err(|e| format!("caddy reload loi: {e}"))?;
    if !status.success() {
        return Err("caddy reload that bai".into());
    }
    Ok(())
}

pub fn stop() -> Result<(), String> {
    let status = caddy_cmd()?
        .arg("stop")
        .status()
        .map_err(|e| format!("caddy stop loi: {e}"))?;
    if !status.success() {
        return Err("caddy stop that bai".into());
    }
    Ok(())
}

/// Apply: neu dang chay thi reload, neu chua thi start.
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
        // Catch-all giu target mac dinh.
        assert!(out.contains("handle {\n\t\treverse_proxy localhost:3000"));
    }

    #[test]
    fn strip_prefix_uses_handle_path() {
        let routes = vec![PathRoute {
            path: "admin".into(), // khong co dau "/" o dau -> tu them
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

/// Cai local CA cua Caddy vao system trust store (`caddy trust`).
/// Chi can chay 1 lan; can quyen admin de ghi vao trust store he thong.
pub fn trust() -> Result<(), String> {
    let bin = find_caddy().ok_or_else(|| {
        "Khong tim thay caddy binary. Cai caddy hoac dat sidecar canh app.".to_string()
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
            .map_err(|e| format!("Chay osascript loi: {e}"))?;
        if !status.success() {
            return Err("caddy trust that bai (cap quyen admin bi tu choi?)".into());
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let status = Command::new("pkexec")
            .arg(bin.as_os_str())
            .arg("trust")
            .status()
            .map_err(|e| format!("Chay pkexec loi: {e}"))?;
        if !status.success() {
            return Err("caddy trust that bai (cap quyen admin bi tu choi?)".into());
        }
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        let status = Command::new(&bin)
            .arg("trust")
            .status()
            .map_err(|e| format!("caddy trust loi: {e}"))?;
        if !status.success() {
            return Err("caddy trust that bai".into());
        }
        Ok(())
    }
}
