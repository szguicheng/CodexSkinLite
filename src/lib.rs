#[cfg(not(target_os = "macos"))]
compile_error!("CodexSkinLite supports only macOS Apple Silicon");

pub mod cdp;
pub mod launcher;
pub mod model;
pub mod paths;
pub mod settings;
pub mod theme;
