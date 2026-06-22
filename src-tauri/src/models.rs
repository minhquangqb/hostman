use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn default_tld() -> String {
    "test".to_string()
}

/// A path-based route within a single domain, e.g. "/admin" -> "localhost:4000".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRoute {
    /// Path prefix, e.g. "/admin".
    pub path: String,
    /// Reverse proxy target for this path, e.g. "localhost:4000".
    pub target: String,
    /// Strip the path prefix before proxying (handle_path instead of handle).
    #[serde(default, rename = "stripPrefix")]
    pub strip_prefix: bool,
}

/// A dev host: domain -> target (host:port).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub id: String,
    /// Short display name, e.g. "myapp".
    pub name: String,
    /// Full domain, e.g. "myapp.test".
    pub domain: String,
    /// Default reverse proxy target (catch-all), e.g. "localhost:2222".
    pub target: String,
    #[serde(default = "default_true")]
    pub https: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Path-based routes pointing to separate targets (e.g. /admin -> a different port).
    #[serde(default)]
    pub paths: Vec<PathRoute>,
}

/// Config synced via git (single source of truth).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "defaultTld", default = "default_tld")]
    pub default_tld: String,
    #[serde(default)]
    pub hosts: Vec<Host>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_tld: default_tld(),
            hosts: Vec::new(),
        }
    }
}

/// Status of the Caddy proxy for display in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaddyStatus {
    pub running: bool,
    /// Path to the caddy binary in use (if found).
    pub binary: Option<String>,
}

/// Status of the background service (launchd / Windows service).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    /// Whether the current OS supports running Caddy as a service.
    pub supported: bool,
    /// Whether the service is installed (plist exists).
    pub installed: bool,
    /// Whether Caddy is running (via the admin API).
    pub running: bool,
}

/// Git status for display in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    /// Whether the config directory is a git repo.
    pub is_repo: bool,
    /// Whether there are uncommitted changes.
    pub dirty: bool,
    /// Number of commits ahead/behind the remote (if any).
    pub ahead: i32,
    pub behind: i32,
    /// Remote URL if present.
    pub remote: Option<String>,
}
