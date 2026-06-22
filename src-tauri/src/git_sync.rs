use crate::config;
use crate::models::GitStatus;
use std::path::PathBuf;
use std::process::Command;

fn repo_dir() -> Result<PathBuf, String> {
    config::config_dir()
}

fn git(args: &[&str]) -> Result<std::process::Output, String> {
    let dir = repo_dir()?;
    Command::new("git")
        .current_dir(&dir)
        .args(args)
        .output()
        .map_err(|e| format!("Chay git loi: {e}"))
}

fn git_ok(args: &[&str]) -> Result<String, String> {
    let out = git(args)?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn is_repo() -> bool {
    git(&["rev-parse", "--is-inside-work-tree"])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Khoi tao git repo tai thu muc config (neu chua co).
pub fn init() -> Result<(), String> {
    config::ensure_dir()?;
    if is_repo() {
        return Ok(());
    }
    git_ok(&["init"])?;
    git_ok(&["add", "-A"])?;
    // Commit dau tien co the loi neu chua co user.name/email -> bo qua loi commit.
    let _ = git(&["commit", "-m", "chore: init hostman config"]);
    Ok(())
}

/// Gan remote origin.
pub fn set_remote(url: &str) -> Result<(), String> {
    if git(&["remote", "get-url", "origin"])
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        git_ok(&["remote", "set-url", "origin", url])?;
    } else {
        git_ok(&["remote", "add", "origin", url])?;
    }
    Ok(())
}

pub fn status() -> GitStatus {
    if !is_repo() {
        return GitStatus {
            is_repo: false,
            dirty: false,
            ahead: 0,
            behind: 0,
            remote: None,
        };
    }
    let dirty = git_ok(&["status", "--porcelain"])
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let remote = git_ok(&["remote", "get-url", "origin"]).ok().filter(|s| !s.is_empty());

    let (ahead, behind) = git_ok(&["rev-list", "--left-right", "--count", "@{upstream}...HEAD"])
        .ok()
        .and_then(|s| {
            let mut parts = s.split_whitespace();
            let behind = parts.next()?.parse::<i32>().ok()?;
            let ahead = parts.next()?.parse::<i32>().ok()?;
            Some((ahead, behind))
        })
        .unwrap_or((0, 0));

    GitStatus {
        is_repo: true,
        dirty,
        ahead,
        behind,
        remote,
    }
}

/// Commit toan bo thay doi.
pub fn commit(message: &str) -> Result<(), String> {
    git_ok(&["add", "-A"])?;
    let out = git(&["commit", "-m", message])?;
    // "nothing to commit" khong phai loi.
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stdout);
        if err.contains("nothing to commit") {
            return Ok(());
        }
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(())
}

pub fn pull() -> Result<String, String> {
    git_ok(&["pull", "--rebase", "--autostash"])
}

pub fn push() -> Result<String, String> {
    // -u de set upstream lan dau.
    git_ok(&["push", "-u", "origin", "HEAD"])
}
