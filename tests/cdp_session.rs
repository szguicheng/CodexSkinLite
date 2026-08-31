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
