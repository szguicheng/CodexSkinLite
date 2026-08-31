#[cfg(not(target_os = "macos"))]
compile_error!("CodexSkinLite supports only macOS Apple Silicon");

pub mod cdp;
pub mod controller;
pub mod diagnostics;
pub mod launcher;
pub mod macos;
pub mod model;
pub mod paths;
pub mod renderer;
pub mod settings;
pub mod theme;
