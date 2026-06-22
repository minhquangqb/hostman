use crate::models::Config;
use std::fs;
use std::path::PathBuf;

const BEGIN_MARKER: &str = "# >>> hostman managed >>>";
const END_MARKER: &str = "# <<< hostman managed <<<";

/// Duong dan hosts file theo OS.
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

/// Tao noi dung block managed tu config.
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

/// Bo block managed cu khoi noi dung hosts file.
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

/// Render noi dung hosts file moi (giu nguyen phan ngoai block, thay block managed).
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

/// Ghi hosts file voi quyen admin (gom 1 lan xin quyen).
pub fn write_hosts_elevated(content: &str) -> Result<(), String> {
    let tmp = std::env::temp_dir().join("hostman_hosts.tmp");
    fs::write(&tmp, content).map_err(|e| format!("Ghi temp loi: {e}"))?;
    let target = hosts_path();

    #[cfg(target_os = "macos")]
    {
        // osascript xin quyen admin qua dialog he thong.
        let script = format!(
            "do shell script \"cp '{}' '{}'\" with administrator privileges",
            tmp.display(),
            target.display()
        );
        let status = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status()
            .map_err(|e| format!("Chay osascript loi: {e}"))?;
        if !status.success() {
            return Err("Cap quyen admin bi tu choi hoac that bai".into());
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Start-Process -Verb RunAs bat UAC.
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
            .map_err(|e| format!("Chay powershell loi: {e}"))?;
        if !status.success() {
            return Err("Cap quyen admin bi tu choi hoac that bai".into());
        }
    }

    #[cfg(target_os = "linux")]
    {
        let status = std::process::Command::new("pkexec")
            .args([
                "cp",
                tmp.to_str().ok_or("tmp path khong hop le")?,
                target.to_str().ok_or("target path khong hop le")?,
            ])
            .status()
            .map_err(|e| format!("Chay pkexec loi: {e}"))?;
        if !status.success() {
            return Err("Cap quyen admin bi tu choi hoac that bai".into());
        }
    }

    let _ = fs::remove_file(&tmp);
    Ok(())
}

/// Ap dung config len hosts file (render + ghi co quyen admin).
pub fn apply(cfg: &Config) -> Result<(), String> {
    let content = render_hosts(cfg)?;
    write_hosts_elevated(&content)
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
