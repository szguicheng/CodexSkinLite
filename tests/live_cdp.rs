#[tokio::test]
#[ignore = "requires a locally running Codex CDP endpoint"]
async fn probe_current_codex_dom() {
    let targets = codex_skin_lite::cdp::list_targets(9222).await.unwrap();
    let target = codex_skin_lite::cdp::pick_primary_target(&targets).unwrap();
    let websocket = target.web_socket_debugger_url.as_deref().unwrap();
    let session = codex_skin_lite::cdp::CdpSession::connect(websocket)
        .await
        .unwrap();
    let value = session
        .evaluate(
            r#"JSON.stringify({
              url: location.href,
              main: !!document.querySelector('main[data-app-shell-main-surface], main.main-surface, main[class*="_MainContentSurface_"]'),
              header: !!document.querySelector('header[data-pip-obstacle="app-shell-header"], header[data-app-shell-header-layout], header[data-app-shell-header-edge-scroll], header[class*="_Header_"], header.app-header-tint'),
              thread: !!document.querySelector('.thread-scroll-container[data-app-action-timeline-scroll], .thread-scroll-container'),
              composer: !!document.querySelector('[data-composer-surface-variant][data-composer-radius-variant], [class*="_ComposerLayoutRoot_"], .composer-surface-chrome'),
              content: !!document.querySelector('[data-csl-thread-content], [class*="max-w-(--thread-content-max-width)"]')
            })"#,
        )
        .await
        .unwrap();
    println!("{}", value.as_str().unwrap());
}

#[tokio::test]
#[ignore = "temporarily injects and cleans the local Codex renderer"]
async fn injects_centered_width_into_current_codex_and_cleans_up() {
    let targets = codex_skin_lite::cdp::list_targets(9222).await.unwrap();
    let target = codex_skin_lite::cdp::pick_primary_target(&targets).unwrap();
    let session = codex_skin_lite::cdp::CdpSession::connect(
        target.web_socket_debugger_url.as_deref().unwrap(),
    )
    .await
    .unwrap();
    session
        .evaluate(
            "window.__CODEX_SKIN_LITE__?.cleanup?.(); delete window.__CODEX_SKIN_LITE__; true",
        )
        .await
        .unwrap();
    session
        .install_bootstrap(codex_skin_lite::renderer::bootstrap_script())
        .await
        .unwrap();
    let status = session
        .apply_payload(&serde_json::json!({
            "revision": 987654,
            "themeEnabled": false,
            "theme": null,
            "conversationCentered": true,
            "conversationMaxWidth": 777
        }))
        .await
        .unwrap();
    assert_eq!(status["revision"], 987654);
    let styles = session
        .evaluate(
            r#"JSON.stringify([
              document.querySelector('[data-csl-thread-content], [class*="max-w-(--thread-content-max-width)"]')?.style.maxWidth,
              document.querySelector('[data-composer-surface-variant][data-composer-radius-variant], [class*="_ComposerLayoutRoot_"], .composer-surface-chrome')?.style.maxWidth
            ])"#,
        )
        .await
        .unwrap();
    assert_eq!(styles.as_str().unwrap(), r#"["777px","777px"]"#);
    session
        .evaluate("window.__CODEX_SKIN_LITE__.cleanup(); true")
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "temporarily injects the locally imported theme and cleans it"]
async fn injects_current_theme_into_current_codex_and_cleans_up() {
    let paths = codex_skin_lite::paths::AppPaths::discover().unwrap();
    let settings = codex_skin_lite::settings::SettingsStore::new(paths.clone())
        .load()
        .unwrap();
    let theme_id = settings.active_theme_id.as_deref().unwrap();
    let theme = codex_skin_lite::theme::ThemeStore::new(paths)
        .load_payload(theme_id)
        .unwrap();
    let targets = codex_skin_lite::cdp::list_targets(9222).await.unwrap();
    let target = codex_skin_lite::cdp::pick_primary_target(&targets).unwrap();
    let session = codex_skin_lite::cdp::CdpSession::connect(
        target.web_socket_debugger_url.as_deref().unwrap(),
    )
    .await
    .unwrap();
    session
        .evaluate(
            "window.__CODEX_SKIN_LITE__?.cleanup?.(); delete window.__CODEX_SKIN_LITE__; true",
        )
        .await
        .unwrap();
    session
        .install_bootstrap(codex_skin_lite::renderer::bootstrap_script())
        .await
        .unwrap();
    let status = session
        .apply_payload(
            &serde_json::to_value(codex_skin_lite::renderer::RendererPayload {
                revision: 987655,
                theme_enabled: true,
                theme: Some(theme),
                conversation_centered: false,
                conversation_max_width: 900,
            })
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status["revision"], 987655);
    let applied = session
        .evaluate(
            r#"JSON.stringify({
              style: !!document.querySelector('#codex-skin-lite-theme'),
              main: !!document.querySelector('[data-ds-part="main"]'),
              image: document.documentElement.style.getPropertyValue('--ds-theme-background-image').startsWith('url("blob:')
            })"#,
        )
        .await
        .unwrap();
    assert_eq!(
        applied.as_str().unwrap(),
        r#"{"style":true,"main":true,"image":true}"#
    );
    session
        .evaluate("window.__CODEX_SKIN_LITE__.cleanup(); true")
        .await
        .unwrap();
}
