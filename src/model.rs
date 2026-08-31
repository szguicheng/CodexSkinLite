use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct AppSettings {
    pub codex_app_path: PathBuf,
    pub debug_port: u16,
    pub theme_enabled: bool,
    pub active_theme_id: Option<String>,
    pub conversation_centered: bool,
    pub conversation_max_width: u16,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            codex_app_path: PathBuf::from("/Applications/Codex.app"),
            debug_port: 9222,
            theme_enabled: false,
            active_theme_id: None,
            conversation_centered: false,
            conversation_max_width: 900,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    RestartRequired,
    CompatibilityWarning(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSnapshot {
    pub settings: AppSettings,
    pub connection: ConnectionState,
    pub active_theme_name: Option<String>,
}
