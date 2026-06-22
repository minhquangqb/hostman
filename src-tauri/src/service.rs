//! Run Caddy as a background service (auto-start, no repeated privilege prompts).
//!
//! macOS: LaunchDaemon at `/Library/LaunchDaemons/com.hostman.caddy.plist`
//! (runs as root so it can bind 80/443 and starts automatically on boot).
//!
//! Windows: Scheduled Task "Hostman Caddy" running under the SYSTEM account
//! (RunLevel = HighestAvailable) with a boot trigger (BootTrigger).
//! Runs elevated so it can bind 80/443 and starts automatically with Windows.
//!
//! Other operating systems: not yet supported (returns a clear error).

use crate::caddy;
use crate::config;
use crate::models::ServiceStatus;

#[cfg(target_os = "macos")]
const LABEL: &str = "com.hostman.caddy";

#[cfg(target_os = "windows")]
const TASK_NAME: &str = "Hostman Caddy";

#[cfg(target_os = "macos")]
fn plist_system_path() -> std::path::PathBuf {
    std::path::PathBuf::from(format!("/Library/LaunchDaemons/{LABEL}.plist"))
}

/// Service status to display in the UI.
pub fn status() -> ServiceStatus {
    #[cfg(target_os = "macos")]
    {
        ServiceStatus {
            supported: true,
            installed: plist_system_path().exists(),
            running: caddy::is_running(),
        }
    }
    #[cfg(target_os = "windows")]
    {
        ServiceStatus {
            supported: true,
            installed: task_exists(),
            running: caddy::is_running(),
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        ServiceStatus {
            supported: false,
            installed: false,
            running: caddy::is_running(),
        }
    }
}

/// Install the service: write the Caddyfile, generate the service definition, register and start it.
/// Requests admin privileges only once.
pub fn install() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let cfg = config::load_config()?;
        let caddyfile = caddy::write_caddyfile(&cfg)?;
        let bin = caddy::find_caddy().ok_or_else(|| {
            "Caddy binary not found. Install caddy or place the sidecar next to the app.".to_string()
        })?;

        let log = config::base_dir()?.join("caddy.log");
        let err_log = config::base_dir()?.join("caddy.err.log");

        let plist = render_plist(
            &bin.display().to_string(),
            &caddyfile.display().to_string(),
            &log.display().to_string(),
            &err_log.display().to_string(),
        );

        // Write a temporary plist into the user directory (no privileges required).
        let staged = config::base_dir()?.join(format!("{LABEL}.plist"));
        std::fs::write(&staged, &plist).map_err(|e| format!("Failed to write temporary plist: {e}"))?;

        let dst = plist_system_path();
        let dst = dst.display().to_string();
        let staged = staged.display().to_string();

        // Combine all privileged operations into a single admin command.
        // bootout first to avoid the "already loaded" error on reinstall.
        let shell = format!(
            "cp '{staged}' '{dst}' && chown root:wheel '{dst}' && chmod 644 '{dst}' && \
             launchctl bootout system/{LABEL} 2>/dev/null; \
             launchctl bootstrap system '{dst}'"
        );
        run_admin_shell(&shell)?;
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        let cfg = config::load_config()?;
        let caddyfile = caddy::write_caddyfile(&cfg)?;
        let bin = caddy::find_caddy().ok_or_else(|| {
            "Caddy binary not found. Install caddy or place the sidecar next to the app.".to_string()
        })?;
        let workdir = config::config_dir()?;

        // Generate the task definition as XML (avoids nested-quote escaping when
        // the path contains spaces, e.g. "C:\Program Files\...").
        let xml = render_task_xml(
            &bin.display().to_string(),
            &caddyfile.display().to_string(),
            &workdir.display().to_string(),
        );
        let xml_path = config::base_dir()?.join("hostman-caddy-task.xml");
        write_utf16le_bom(&xml_path, &xml)
            .map_err(|e| format!("Failed to write task XML file: {e}"))?;

        // Create (overwrite) the task from XML, then run it immediately. Combined into a single elevation.
        let bat = format!(
            "@echo off\r\n\
             schtasks /Create /TN \"{TASK_NAME}\" /XML \"{xml}\" /F\r\n\
             if errorlevel 1 exit /b 1\r\n\
             schtasks /Run /TN \"{TASK_NAME}\"\r\n\
             exit /b 0\r\n",
            xml = xml_path.display()
        );
        run_admin_bat("hostman-service-install.bat", &bat)?;
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Running Caddy as a service is currently only supported on macOS and Windows.".into())
    }
}

/// Uninstall the service: stop it and remove the registration.
pub fn uninstall() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let dst = plist_system_path().display().to_string();
        let shell = format!("launchctl bootout system/{LABEL} 2>/dev/null; rm -f '{dst}'");
        run_admin_shell(&shell)?;
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        // /End stops the running instance (kills caddy), then /Delete removes the task.
        let bat = format!(
            "@echo off\r\n\
             schtasks /End /TN \"{TASK_NAME}\" >nul 2>&1\r\n\
             schtasks /Delete /TN \"{TASK_NAME}\" /F\r\n\
             exit /b 0\r\n"
        );
        run_admin_bat("hostman-service-uninstall.bat", &bat)?;
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Running Caddy as a service is currently only supported on macOS and Windows.".into())
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

/// Run a shell script with admin privileges via the system dialog (macOS).
#[cfg(target_os = "macos")]
fn run_admin_shell(cmd: &str) -> Result<(), String> {
    use std::process::Command;
    // Escape for the AppleScript string.
    let escaped = cmd.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!("do shell script \"{escaped}\" with administrator privileges");
    let status = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .status()
        .map_err(|e| format!("Failed to run osascript: {e}"))?;
    if !status.success() {
        return Err("Admin operation failed (privileges denied?)".into());
    }
    Ok(())
}

// ---------- Windows helpers ----------

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Whether the task is already registered (no admin privileges needed to query).
#[cfg(target_os = "windows")]
fn task_exists() -> bool {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    Command::new("schtasks")
        .args(["/Query", "/TN", TASK_NAME])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Generate the Scheduled Task XML that runs caddy under SYSTEM at boot.
///
/// - `S-1-5-18`: SID of the SYSTEM account (using the SID avoids dependence on the OS language).
/// - `RunLevel = HighestAvailable`: run elevated (bind 80/443).
/// - `ExecutionTimeLimit = PT0S`: no time limit (caddy runs indefinitely).
#[cfg(target_os = "windows")]
fn render_task_xml(bin: &str, caddyfile: &str, workdir: &str) -> String {
    let bin = xml_escape(bin);
    let caddyfile = xml_escape(caddyfile);
    let workdir = xml_escape(workdir);
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Hostman - run the Caddy reverse proxy at system boot</Description>
  </RegistrationInfo>
  <Triggers>
    <BootTrigger>
      <Enabled>true</Enabled>
    </BootTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>S-1-5-18</UserId>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RestartOnFailure>
      <Interval>PT1M</Interval>
      <Count>3</Count>
    </RestartOnFailure>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{bin}</Command>
      <Arguments>run --config "{caddyfile}"</Arguments>
      <WorkingDirectory>{workdir}</WorkingDirectory>
    </Exec>
  </Actions>
</Task>
"#
    )
}

#[cfg(target_os = "windows")]
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Write a UTF-16LE file with a BOM — `schtasks /XML` expects a UTF-16 file.
#[cfg(target_os = "windows")]
fn write_utf16le_bom(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    let mut bytes = vec![0xFF, 0xFE]; // BOM UTF-16LE
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(path, bytes)
}

/// Write `script` to a temporary .bat file, then run it with admin privileges via UAC.
/// Uses PowerShell `Start-Process -Verb RunAs -Wait` to trigger the UAC dialog once
/// and wait for the result; the .bat's exit code determines success/failure.
#[cfg(target_os = "windows")]
fn run_admin_bat(file_name: &str, script: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let bat_path = config::base_dir()?.join(file_name);
    std::fs::write(&bat_path, script).map_err(|e| format!("Failed to write .bat file: {e}"))?;

    // PowerShell single-quoted string: escape ' as ''.
    let p = bat_path.display().to_string().replace('\'', "''");
    let ps = format!(
        "$ErrorActionPreference='Stop'; \
         $proc = Start-Process -FilePath '{p}' -Verb RunAs -Wait -PassThru -WindowStyle Hidden; \
         exit $proc.ExitCode"
    );

    let status = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("Failed to run PowerShell: {e}"))?;
    if !status.success() {
        return Err("Admin operation failed (UAC denied?)".into());
    }
    Ok(())
}
