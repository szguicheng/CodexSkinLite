fn main() -> anyhow::Result<()> {
    if std::env::consts::ARCH != "aarch64" {
        anyhow::bail!("CodexSkinLite supports only macOS Apple Silicon");
    }
    let paths = codex_skin_lite::paths::AppPaths::discover()?;
    let _diagnostics = codex_skin_lite::diagnostics::init_local_logging(&paths)?;
    let state = std::sync::Arc::new(codex_skin_lite::macos::AppKitState::default());
    let sink = std::sync::Arc::new(codex_skin_lite::macos::AppKitSink::new(state.clone()));
    let controller = codex_skin_lite::controller::Controller::new(
        codex_skin_lite::settings::SettingsStore::new(paths.clone()),
        codex_skin_lite::theme::ThemeStore::new(paths),
        std::sync::Arc::new(codex_skin_lite::controller::NativeRuntime::default()),
        sink,
    )?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("codex-skin-lite")
        .build()?;
    let handle = {
        let _guard = runtime.enter();
        codex_skin_lite::controller::spawn(controller)
    };
    codex_skin_lite::macos::run(handle, state)
}
