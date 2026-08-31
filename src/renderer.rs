use serde::Serialize;

use crate::theme::ThemePayload;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RendererPayload {
    pub revision: u64,
    pub theme_enabled: bool,
    pub theme: Option<ThemePayload>,
    pub conversation_centered: bool,
    pub conversation_max_width: u16,
}

pub fn bootstrap_script() -> &'static str {
    include_str!("../assets/renderer/runtime.js")
}
