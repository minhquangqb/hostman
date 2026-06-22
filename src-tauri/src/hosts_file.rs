use crate::models::Config;
use std::fs;
use std::path::PathBuf;

const BEGIN_MARKER: &str = "# >>> hostman managed >>>";
const END_MARKER: &str = "# <<< hostman managed <<<";

/// Path to the hosts file for the current OS.
pub fn hosts_path() -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/etc/hosts")
    }
}

/// Build the managed block content from the config.
fn build_block(cfg: &Config) -> String {
    let mut lines = vec![BEGIN_MARKER.to_string()];
    for h in &cfg.hosts {
        if h.enabled {
            lines.push(format!("127.0.0.1\t{}", h.domain));
            lines.push(format!("::1\t{}", h.domain));
        }
    }
    lines.push(END_MARKER.to_string());
    lines.join("\n")
}

/// Remove the existing managed block from the hosts file content.
fn strip_block(content: &str) -> String {
    let mut out = String::new();
    let mut in_block = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == BEGIN_MARKER {
            in_block = true;
            continue;
        }
        if trimmed == END_MARKER {
            in_block = false;
            continue;
        }
        if !in_block {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Render the new hosts file content (preserving everything outside the block, replacing the managed block).
pub fn render_hosts(cfg: &Config) -> Result<String, String> {
    let path = hosts_path();
    let current = fs::read_to_string(&path).unwrap_or_default();
    let stripped = strip_block(&current);
    let block = build_block(cfg);
    let mut out = stripped.trim_end().to_string();
    out.push_str("\n\n");
    out.push_str(&block);
    out.push('\n');
    Ok(out)
}

/// Write the hosts file with admin privileges (prompting for privileges only once).
pub fn write_hosts_elevated(content: &str) -> Result<(), String> {
    // Give the temp file a unique name (pid + nanos) to avoid two writes
    // overwriting each other -> prevents corrupting /etc/hosts (e.g. garbage
    // bytes leaking into the start of the file).
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = std::env::temp_dir().join(format!(
        "hostman_hosts_{}_{unique}.tmp",
        std::process::id()
    ));
    fs::write(&tmp, content).map_err(|e| format!("Failed to write temp file: {e}"))?;
    let target = hosts_path();

    #[cfg(target_os = "macos")]
    {
        // osascript requests admin privileges via the system dialog.
        let script = format!(
            "do shell script \"cp '{}' '{}'\" with administrator privileges",
            tmp.display(),
            target.display()
        );
        let status = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status()
            .map_err(|e| format!("Failed to run osascript: {e}"))?;
        if !status.success() {
            return Err("Admin privileges were denied or the operation failed".into());
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Start-Process -Verb RunAs triggers the UAC prompt.
        let inner = format!(
            "copy /Y \"{}\" \"{}\"",
            tmp.display(),
            target.display()
        );
        let cmd = format!(
            "Start-Process -Verb RunAs -Wait -WindowStyle Hidden -FilePath cmd -ArgumentList '/c {}'",
            inner
        );
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &cmd])
            .status()
            .map_err(|e| format!("Failed to run powershell: {e}"))?;
        if !status.success() {
            return Err("Admin privileges were denied or the operation failed".into());
        }
    }

    #[cfg(target_os = "linux")]
    {
        let status = std::process::Command::new("pkexec")
            .args([
                "cp",
                tmp.to_str().ok_or("invalid tmp path")?,
                target.to_str().ok_or("invalid target path")?,
            ])
            .status()
            .map_err(|e| format!("Failed to run pkexec: {e}"))?;
        if !status.success() {
            return Err("Admin privileges were denied or the operation failed".into());
        }
    }

    let _ = fs::remove_file(&tmp);
    Ok(())
}

/// Apply the config to the hosts file (render + write with admin privileges).
pub fn apply(cfg: &Config) -> Result<(), String> {
    let content = render_hosts(cfg)?;
    write_hosts_elevated(&content)
}

/// Open the hosts file in the system's default text editor.
pub fn open_in_editor() -> Result<(), String> {
    let path = hosts_path();

    #[cfg(target_os = "macos")]
    let mut cmd = {
        // `open -t` opens the file in the default text editor.
        let mut c = std::process::Command::new("open");
        c.arg("-t").arg(&path);
        c
    };

    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("notepad");
        c.arg(&path);
        c
    };

    #[cfg(target_os = "linux")]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(&path);
        c
    };

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to open the hosts file: {e}"))?;
    if !status.success() {
        return Err("Failed to open the hosts file".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Host;

    fn host(domain: &str, enabled: bool) -> Host {
        Host {
            id: "1".into(),
            name: "x".into(),
            domain: domain.into(),
            target: "localhost:3000".into(),
            https: true,
            enabled,
            paths: vec![],
        }
    }

    #[test]
    fn strip_removes_managed_block() {
        let input = format!(
            "127.0.0.1 localhost\n{BEGIN_MARKER}\n127.0.0.1 old.test\n{END_MARKER}\n# comment\n"
        );
        let out = strip_block(&input);
        assert!(out.contains("127.0.0.1 localhost"));
        assert!(out.contains("# comment"));
        assert!(!out.contains("old.test"));
    }

    #[test]
    fn build_block_only_enabled() {
        let cfg = Config {
            default_tld: "test".into(),
            hosts: vec![host("a.test", true), host("b.test", false)],
        };
        let block = build_block(&cfg);
        assert!(block.contains("a.test"));
        assert!(!block.contains("b.test"));
    }
}
