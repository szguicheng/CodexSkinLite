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
              document.querySelector('[data-pip-obstacle="thread-footer"]')?.style.maxWidth,
              document.querySelector('[data-composer-surface-variant][data-composer-radius-variant], [class*="_ComposerLayoutRoot_"], .composer-surface-chrome')?.style.maxWidth
            ])"#,
        )
        .await
        .unwrap();
    assert_eq!(styles.as_str().unwrap(), r#"["777px","777px",""]"#);
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
                conversation_centered: true,
                conversation_max_width: 777,
            })
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status["revision"], 987655);
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    let applied = session
        .evaluate(
            r#"(() => {
              const main = document.querySelector('main[data-app-shell-main-surface], main.main-surface, main[class*="_MainContentSurface_"]');
              const thread = main?.querySelector('.thread-scroll-container[data-app-action-timeline-scroll], .thread-scroll-container');
              const footers = [...(thread?.querySelectorAll('[data-thread-scroll-footer]') || [])];
              const composers = [...(thread?.querySelectorAll('[data-composer-surface-variant][data-composer-radius-variant], [class*="_ComposerLayoutRoot_"], .composer-surface-chrome') || [])];
              const footer = footers[0];
              const composer = composers[0];
              const titleSurface = document.querySelector('[data-csl-header-title-surface="true"]');
              return JSON.stringify({
                apiVersion: window.__CODEX_SKIN_LITE__?.apiVersion,
                style: !!document.querySelector('#codex-skin-lite-theme'),
                main: !!main && main.getAttribute('data-ds-part') === 'main',
                image: document.documentElement.style.getPropertyValue('--ds-theme-background-image').startsWith('url("blob:'),
                footerCount: footers.length,
                composerCount: composers.length,
                footerInThread: !!footer && !!thread && thread.contains(footer),
                composerInsideThread: !!composer && !!thread && thread.contains(composer),
                footerDocked: footer?.dataset.cslComposerDock === 'true' && footer.parentElement?.hasAttribute('data-app-shell-main-content-layout'),
                footerPosition: footer ? getComputedStyle(footer).position : null,
                footerWidth: footer?.querySelector('[data-pip-obstacle="thread-footer"]')?.style.maxWidth,
                titleTransparent: !!titleSurface && getComputedStyle(titleSurface).backgroundColor === 'rgba(0, 0, 0, 0)'
              });
            })()"#,
        )
        .await
        .unwrap();
    assert_eq!(
        applied.as_str().unwrap(),
        r#"{"apiVersion":6,"style":true,"main":true,"image":true,"footerCount":1,"composerCount":1,"footerInThread":true,"composerInsideThread":true,"footerDocked":false,"footerPosition":"fixed","footerWidth":"777px","titleTransparent":true}"#
    );
    let scroll_geometry = session
        .evaluate(
            r#"(() => {
              const main = document.querySelector('main[data-app-shell-main-surface], main.main-surface, main[class*="_MainContentSurface_"]');
              const thread = main?.querySelector('.thread-scroll-container[data-app-action-timeline-scroll], .thread-scroll-container');
              const footer = thread?.querySelector('[data-thread-scroll-footer]');
              if (!thread || !footer) return JSON.stringify({ missing: true });
              const originalScrollTop = thread.scrollTop;
              const before = footer.getBoundingClientRect();
              let changedScrollTop = originalScrollTop;
              for (const delta of [-320, 320]) {
                thread.scrollTop = originalScrollTop + delta;
                changedScrollTop = thread.scrollTop;
                if (Math.abs(changedScrollTop - originalScrollTop) > 1) break;
              }
              const after = footer.getBoundingClientRect();
              const threadRect = thread.getBoundingClientRect();
              thread.scrollTop = originalScrollTop;
              return JSON.stringify({
                scrollChanged: Math.abs(changedScrollTop - originalScrollTop) > 1,
                leftDelta: after.left - before.left,
                topDelta: after.top - before.top,
                bottomDelta: after.bottom - before.bottom,
                bottomGap: threadRect.bottom - after.bottom
              });
            })()"#,
        )
        .await
        .unwrap();
    let geometry: serde_json::Value =
        serde_json::from_str(scroll_geometry.as_str().unwrap()).unwrap();
    assert_eq!(
        geometry["scrollChanged"], true,
        "thread did not scroll: {geometry}"
    );
    for key in ["leftDelta", "topDelta", "bottomDelta", "bottomGap"] {
        let value = geometry[key].as_f64().unwrap();
        assert!(value.abs() <= 1.0, "{key} changed by {value}: {geometry}");
    }
    tokio::time::sleep(std::time::Duration::from_millis(750)).await;
    let settled = session
        .evaluate("JSON.stringify(window.__CODEX_SKIN_LITE__.status().metrics)")
        .await
        .unwrap();
    let metrics: serde_json::Value = serde_json::from_str(settled.as_str().unwrap()).unwrap();
    assert!(
        metrics["layoutPasses"].as_u64().unwrap() <= 20,
        "renderer did not settle: {metrics}"
    );
    session
        .evaluate("window.__CODEX_SKIN_LITE__.cleanup(); true")
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "prints the current Codex layout geometry for diagnosis"]
async fn probe_current_composer_and_header_geometry() {
    let targets = codex_skin_lite::cdp::list_targets(9222).await.unwrap();
    let target = codex_skin_lite::cdp::pick_primary_target(&targets).unwrap();
    let session = codex_skin_lite::cdp::CdpSession::connect(
        target.web_socket_debugger_url.as_deref().unwrap(),
    )
    .await
    .unwrap();
    let value = session
        .evaluate(
            r#"(() => {
              const describe = (node) => {
                if (!node) return null;
                const style = getComputedStyle(node);
                const rect = node.getBoundingClientRect();
                return {
                  tag: node.tagName,
                  cls: String(node.className || ''),
                  attrs: Object.fromEntries([...node.attributes].map(a => [a.name, a.value])),
                  rect: { left: rect.left, right: rect.right, top: rect.top, bottom: rect.bottom, width: rect.width, height: rect.height },
                  style: {
                    position: style.position,
                    overflowY: style.overflowY,
                    backgroundColor: style.backgroundColor,
                    backdropFilter: style.backdropFilter,
                    maxWidth: style.maxWidth,
                    marginLeft: style.marginLeft,
                    marginRight: style.marginRight,
                    zIndex: style.zIndex
                  }
                };
              };
              const composer = document.querySelector('[data-composer-surface-variant][data-composer-radius-variant], [class*="_ComposerLayoutRoot_"], .composer-surface-chrome');
              const thread = document.querySelector('.thread-scroll-container[data-app-action-timeline-scroll], .thread-scroll-container');
              const content = document.querySelector('[data-csl-thread-content], [class*="max-w-(--thread-content-max-width)"]');
              const main = document.querySelector('main[data-app-shell-main-surface], main.main-surface, main[class*="_MainContentSurface_"]');
              const footers = [...document.querySelectorAll('[data-thread-scroll-footer]')];
              const composers = [...document.querySelectorAll('[data-composer-surface-variant][data-composer-radius-variant], [class*="_ComposerLayoutRoot_"], .composer-surface-chrome')];
              const titleText = [...document.querySelectorAll('header *')].find(node => node.children.length === 0 && node.textContent?.includes('修复 Claude EVA macOS 主题'));
              const chain = node => {
                const result = [];
                for (let current = node, i = 0; current && i < 9; current = current.parentElement, i += 1) result.push(describe(current));
                return result;
              };
              const threadSurfaces = [...document.querySelectorAll('.thread-scroll-container')].map(threadNode => ({
                thread: describe(threadNode),
                ancestors: chain(threadNode).slice(1, 6),
                footerCount: threadNode.querySelectorAll('[data-thread-scroll-footer]').length,
                composerCount: threadNode.querySelectorAll('[data-composer-surface-variant][data-composer-radius-variant], [class*="_ComposerLayoutRoot_"], .composer-surface-chrome').length
              }));
              return JSON.stringify({
                api: window.__CODEX_SKIN_LITE__?.status?.() || null,
                footerCount: footers.length,
                composerCount: composers.length,
                dockedFooterCount: footers.filter(node => node.dataset.cslComposerDock === 'true').length,
                footers: footers.map(node => ({ footer: describe(node), parent: describe(node.parentElement) })),
                composerInsideThread: !!(thread && composer && thread.contains(composer)),
                main: describe(main),
                thread: describe(thread),
                content: describe(content),
                threadSurfaces,
                sidebars: [...document.querySelectorAll('.app-shell-left-panel, [data-app-shell-right-panel], [data-context-panel], aside[class*="_RightPanel_"], [data-app-shell-tabs="true"], [data-browser-sidebar-webview-host-root], [data-browser-sidebar-webview]')].map(describe),
                composerChain: chain(composer),
                titleChain: chain(titleText)
              });
            })()"#,
        )
        .await
        .unwrap();
    println!("{}", value.as_str().unwrap());
}
