use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn default_tld() -> String {
    "test".to_string()
}

/// Mot route theo path trong cung mot domain, vd "/admin" -> "localhost:4000".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRoute {
    /// Path prefix, vd "/admin".
    pub path: String,
    /// Dich reverse proxy cho path nay, vd "localhost:4000".
    pub target: String,
    /// Bo tien to path truoc khi proxy (handle_path thay vi handle).
    #[serde(default, rename = "stripPrefix")]
    pub strip_prefix: bool,
}

/// Mot dev host: domain -> target (host:port).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub id: String,
    /// Ten ngan gon de hien thi, vd "myapp".
    pub name: String,
    /// Domain day du, vd "myapp.test".
    pub domain: String,
    /// Dich reverse proxy mac dinh (catch-all), vd "localhost:2222".
    pub target: String,
    #[serde(default = "default_true")]
    pub https: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Cac route theo path tro toi target rieng (vd /admin -> port khac).
    #[serde(default)]
    pub paths: Vec<PathRoute>,
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

/// Trang thai cua background service (launchd / Windows service).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    /// OS hien tai co ho tro chay Caddy nhu service khong.
    pub supported: bool,
    /// Service da duoc cai (plist ton tai) chua.
    pub installed: bool,
    /// Caddy co dang chay (qua admin API) khong.
    pub running: bool,
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
