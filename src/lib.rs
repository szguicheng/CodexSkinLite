#[cfg(not(target_os = "macos"))]
compile_error!("CodexSkinLite supports only macOS Apple Silicon");

pub mod model;
