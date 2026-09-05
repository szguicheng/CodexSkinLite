use std::time::Duration;

use codex_skin_lite::cdp::{CdpSession, ReconnectBackoff};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn correlates_out_of_order_cdp_results_by_id() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        let first: Value =
            serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
        let second: Value =
            serde_json::from_str(socket.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
        for (request, value) in [(second, 4), (first, 2)] {
            socket
                .send(Message::Text(
                    serde_json::json!({
                        "id": request["id"],
                        "result": { "result": { "value": value } }
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
        }
    });
    let session = CdpSession::connect(&format!("ws://{address}/devtools/page/main"))
        .await
        .unwrap();

    let (left, right) = tokio::join!(session.evaluate("1+1"), session.evaluate("2+2"));

    assert_eq!(left.unwrap(), serde_json::json!(2));
    assert_eq!(right.unwrap(), serde_json::json!(4));
}

#[test]
fn reconnect_backoff_is_bounded_and_resets() {
    let mut value = ReconnectBackoff::default();
    assert_eq!(value.next_delay(), Duration::from_millis(250));
    for _ in 0..20 {
        value.next_delay();
    }
    assert_eq!(value.current(), Duration::from_secs(30));
    value.reset();
    assert_eq!(value.current(), Duration::from_millis(250));
}

#[tokio::test]
async fn detach_removes_bootstrap_cleans_renderer_and_closes_even_if_cleanup_fails() {
    for fail_cleanup in [false, true] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
            let mut methods = Vec::new();
            while let Some(Ok(message)) = socket.next().await {
                if message.is_close() {
                    break;
                }
                let request: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
                let method = request["method"].as_str().unwrap().to_string();
                let result = if method == "Page.addScriptToEvaluateOnNewDocument" {
                    serde_json::json!({"identifier": "owned-bootstrap"})
                } else if method == "Page.removeScriptToEvaluateOnNewDocument" {
                    assert_eq!(request["params"]["identifier"], "owned-bootstrap");
                    serde_json::json!({})
                } else if request["params"]["expression"]
                    .as_str()
                    .unwrap_or("")
                    .contains("cleanup")
                {
                    if fail_cleanup {
                        serde_json::json!({"exceptionDetails":{"text":"cleanup failed"}})
                    } else {
                        serde_json::json!({"result":{"value":true}})
                    }
                } else {
                    serde_json::json!({"result":{"value":true}})
                };
                methods.push(method);
                socket
                    .send(Message::Text(
                        serde_json::json!({"id":request["id"],"result":result})
                            .to_string()
                            .into(),
                    ))
                    .await
                    .unwrap();
            }
            methods
        });
        let session = CdpSession::connect(&format!("ws://{address}/devtools/page/main"))
            .await
            .unwrap();
        session.install_bootstrap("true").await.unwrap();
        assert_eq!(session.detach().await.is_err(), fail_cleanup);
        let methods = tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            methods,
            [
                "Page.addScriptToEvaluateOnNewDocument",
                "Runtime.evaluate",
                "Page.removeScriptToEvaluateOnNewDocument",
                "Runtime.evaluate"
            ]
        );
    }
}
