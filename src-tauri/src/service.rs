//! Chay Caddy nhu mot background service (auto-start, khong xin quyen lap lai).
//!
//! macOS: LaunchDaemon tai `/Library/LaunchDaemons/com.hostman.caddy.plist`
//! (chay duoi quyen root nen bind duoc 80/443 va tu khoi dong khi may bat).
//! Cac OS khac: chua ho tro (tra ve loi ro rang).

use crate::caddy;
use crate::config;
use crate::models::ServiceStatus;

#[cfg(target_os = "macos")]
const LABEL: &str = "com.hostman.caddy";

#[cfg(target_os = "macos")]
fn plist_system_path() -> std::path::PathBuf {
    std::path::PathBuf::from(format!("/Library/LaunchDaemons/{LABEL}.plist"))
}

/// Trang thai service de hien thi tren UI.
pub fn status() -> ServiceStatus {
    #[cfg(target_os = "macos")]
    {
        ServiceStatus {
            supported: true,
            installed: plist_system_path().exists(),
            running: caddy::is_running(),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        ServiceStatus {
            supported: false,
            installed: false,
            running: caddy::is_running(),
        }
    }
}

/// Cai service: ghi Caddyfile, sinh plist, copy vao LaunchDaemons va bootstrap.
/// Chi xin quyen admin 1 lan (gop moi thao tac vao 1 shell script).
pub fn install() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let cfg = config::load_config()?;
        let caddyfile = caddy::write_caddyfile(&cfg)?;
        let bin = caddy::find_caddy().ok_or_else(|| {
            "Khong tim thay caddy binary. Cai caddy hoac dat sidecar canh app.".to_string()
        })?;

        let log = config::base_dir()?.join("caddy.log");
        let err_log = config::base_dir()?.join("caddy.err.log");

        let plist = render_plist(
            &bin.display().to_string(),
            &caddyfile.display().to_string(),
            &log.display().to_string(),
            &err_log.display().to_string(),
        );

        // Ghi plist tam vao thu muc user (khong can quyen).
        let staged = config::base_dir()?.join(format!("{LABEL}.plist"));
        std::fs::write(&staged, &plist).map_err(|e| format!("Ghi plist tam loi: {e}"))?;

        let dst = plist_system_path();
        let dst = dst.display().to_string();
        let staged = staged.display().to_string();

        // Gop tat ca thao tac can quyen vao 1 lenh admin.
        // bootout truoc de tranh loi "already loaded" khi cai lai.
        let shell = format!(
            "cp '{staged}' '{dst}' && chown root:wheel '{dst}' && chmod 644 '{dst}' && \
             launchctl bootout system/{LABEL} 2>/dev/null; \
             launchctl bootstrap system '{dst}'"
        );
        run_admin_shell(&shell)?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Chay Caddy nhu service hien chi ho tro tren macOS.".into())
    }
}

/// Go service: bootout va xoa plist khoi LaunchDaemons.
pub fn uninstall() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let dst = plist_system_path().display().to_string();
        let shell = format!(
            "launchctl bootout system/{LABEL} 2>/dev/null; rm -f '{dst}'"
        );
        run_admin_shell(&shell)?;
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Chay Caddy nhu service hien chi ho tro tren macOS.".into())
    }
}

#[cfg(target_os = "macos")]
fn render_plist(bin: &str, caddyfile: &str, log: &str, err_log: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>{LABEL}</string>
	<key>ProgramArguments</key>
	<array>
		<string>{bin}</string>
		<string>run</string>
		<string>--config</string>
		<string>{caddyfile}</string>
	</array>
	<key>RunAtLoad</key>
	<true/>
	<key>KeepAlive</key>
	<true/>
	<key>StandardOutPath</key>
	<string>{log}</string>
	<key>StandardErrorPath</key>
	<string>{err_log}</string>
</dict>
</plist>
"#
    )
}

/// Chay 1 shell script voi quyen admin qua dialog he thong (macOS).
#[cfg(target_os = "macos")]
fn run_admin_shell(cmd: &str) -> Result<(), String> {
    use std::process::Command;
    // Escape cho chuoi AppleScript.
    let escaped = cmd.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!("do shell script \"{escaped}\" with administrator privileges");
    let status = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .status()
        .map_err(|e| format!("Chay osascript loi: {e}"))?;
    if !status.success() {
        return Err("Thao tac admin that bai (cap quyen bi tu choi?)".into());
    }
    Ok(())
}
