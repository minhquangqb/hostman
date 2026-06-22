use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn default_tld() -> String {
    "test".to_string()
}

/// Mot dev host: domain -> target (host:port).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub id: String,
    /// Ten ngan gon de hien thi, vd "myapp".
    pub name: String,
    /// Domain day du, vd "myapp.test".
    pub domain: String,
    /// Dich reverse proxy, vd "localhost:2222".
    pub target: String,
    #[serde(default = "default_true")]
    pub https: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Config dong bo qua git (single source of truth).
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

/// Trang thai cua Caddy proxy de hien thi tren UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaddyStatus {
    pub running: bool,
    /// Duong dan caddy binary dang dung (neu tim thay).
    pub binary: Option<String>,
}

/// Trang thai git de hien thi tren UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    /// Thu muc config co phai git repo khong.
    pub is_repo: bool,
    /// Co thay doi chua commit khong.
    pub dirty: bool,
    /// So commit ahead/behind so voi remote (neu co).
    pub ahead: i32,
    pub behind: i32,
    /// Remote URL neu co.
    pub remote: Option<String>,
}
