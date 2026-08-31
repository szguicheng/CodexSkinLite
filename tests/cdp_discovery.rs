use codex_skin_lite::cdp::{CdpTarget, list_targets, pick_primary_target, validate_websocket_url};

#[test]
fn selects_primary_codex_target_and_rejects_quick_chat() {
    let targets = vec![
        target("Quick Chat", "file:///quick-chat.html", "page", 1),
        target("Codex", "file:///index.html", "page", 2),
    ];

    assert_eq!(pick_primary_target(&targets).unwrap().title, "Codex");
}

#[test]
fn rejects_non_loopback_websocket_url() {
    let value = "ws://192.168.1.10:9222/devtools/page/1";
    assert!(validate_websocket_url(value, 9222).is_err());
}

#[test]
fn rejects_page_that_only_spoofs_the_codex_title() {
    let targets = vec![target("Codex", "https://evil.example/", "page", 1)];
    assert!(pick_primary_target(&targets).is_err());
}

#[tokio::test]
async fn lists_targets_from_a_loopback_endpoint() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0_u8; 2048];
        let _ = stream.read(&mut request).await.unwrap();
        let body = format!(
            r#"[{{"id":"main","title":"Codex","url":"file:///index.html","type":"page","webSocketDebuggerUrl":"ws://127.0.0.1:{port}/devtools/page/main"}}]"#
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    });

    let targets = list_targets(port).await.unwrap();

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].id, "main");
}

fn target(title: &str, url: &str, kind: &str, id: u8) -> CdpTarget {
    CdpTarget {
        id: id.to_string(),
        title: title.into(),
        url: url.into(),
        kind: kind.into(),
        web_socket_debugger_url: Some(format!("ws://127.0.0.1:9222/devtools/page/{id}")),
    }
}
