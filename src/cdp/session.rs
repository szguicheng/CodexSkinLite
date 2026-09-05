use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;

use super::protocol;

type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<anyhow::Result<Value>>>>>;

#[derive(Clone)]
pub struct CdpSession {
    outbound: mpsc::UnboundedSender<Message>,
    pending: PendingMap,
    next_id: Arc<AtomicU64>,
    bootstrap_ids: Arc<Mutex<Vec<String>>>,
}

impl CdpSession {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let (socket, _) = tokio_tungstenite::connect_async(url).await?;
        let (mut socket_writer, mut socket_reader) = socket.split();
        let (outbound, mut outbound_rx) = mpsc::unbounded_channel::<Message>();
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let writer_pending = pending.clone();
        tokio::spawn(async move {
            while let Some(message) = outbound_rx.recv().await {
                if let Err(error) = socket_writer.send(message).await {
                    fail_pending(&writer_pending, format!("CDP writer closed: {error}")).await;
                    break;
                }
            }
        });
        let reader_pending = pending.clone();
        tokio::spawn(async move {
            while let Some(message) = socket_reader.next().await {
                let message = match message {
                    Ok(message) => message,
                    Err(error) => {
                        fail_pending(&reader_pending, format!("CDP reader closed: {error}")).await;
                        return;
                    }
                };
                let Some(text) = message.to_text().ok() else {
                    if message.is_close() {
                        break;
                    }
                    continue;
                };
                let Ok(value) = serde_json::from_str::<Value>(text) else {
                    continue;
                };
                let Some(id) = value.get("id").and_then(Value::as_u64) else {
                    continue;
                };
                let Some(sender) = reader_pending.lock().await.remove(&id) else {
                    continue;
                };
                let result = if let Some(error) = value.get("error") {
                    Err(anyhow::anyhow!("CDP command failed: {error}"))
                } else {
                    Ok(value.get("result").cloned().unwrap_or(Value::Null))
                };
                let _ = sender.send(result);
            }
            fail_pending(&reader_pending, "CDP socket closed".into()).await;
        });
        Ok(Self {
            outbound,
            pending,
            next_id: Arc::new(AtomicU64::new(1)),
            bootstrap_ids: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub async fn evaluate(&self, expression: &str) -> anyhow::Result<Value> {
        let result = self
            .send_command("Runtime.evaluate", protocol::evaluate_params(expression))
            .await?;
        if let Some(exception) = result.get("exceptionDetails") {
            anyhow::bail!("renderer evaluation failed: {exception}");
        }
        Ok(result
            .pointer("/result/value")
            .cloned()
            .unwrap_or(Value::Null))
    }

    pub async fn install_bootstrap(&self, script: &str) -> anyhow::Result<()> {
        let registration = self
            .send_command(
                "Page.addScriptToEvaluateOnNewDocument",
                json!({ "source": script }),
            )
            .await?;
        let id = registration
            .get("identifier")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("CDP bootstrap registration has no identifier"))?;
        self.bootstrap_ids.lock().await.push(id.to_string());
        self.evaluate(script).await?;
        Ok(())
    }

    pub async fn apply_payload(&self, payload: &Value) -> anyhow::Result<Value> {
        let json = serde_json::to_string(payload)?;
        let quoted_json = serde_json::to_string(&json)?;
        self.evaluate(&format!(
            "window.__CODEX_SKIN_LITE__.apply(JSON.parse({quoted_json}))"
        ))
        .await
    }

    pub async fn close(self) -> anyhow::Result<()> {
        self.outbound
            .send(Message::Close(None))
            .map_err(|_| anyhow::anyhow!("CDP session is already closed"))?;
        Ok(())
    }

    pub async fn detach(self) -> anyhow::Result<()> {
        let ids = std::mem::take(&mut *self.bootstrap_ids.lock().await);
        let mut outcome = Ok(());
        for id in ids {
            if let Err(error) = self
                .send_command(
                    "Page.removeScriptToEvaluateOnNewDocument",
                    json!({"identifier": id}),
                )
                .await
            {
                outcome = Err(error);
            }
        }
        if let Err(error) = self
            .evaluate(
                "window.__CODEX_SKIN_LITE__?.cleanup?.(); delete window.__CODEX_SKIN_LITE__; true",
            )
            .await
        {
            outcome = Err(error);
        }
        let closed = self.close().await;
        outcome?;
        closed
    }

    async fn send_command(&self, method: &str, params: Value) -> anyhow::Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().await.insert(id, sender);
        let message = Message::Text(protocol::command(id, method, params).to_string().into());
        if self.outbound.send(message).is_err() {
            self.pending.lock().await.remove(&id);
            anyhow::bail!("CDP session is closed");
        }
        match tokio::time::timeout(Duration::from_secs(10), receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => anyhow::bail!("CDP response channel closed"),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                anyhow::bail!("CDP command timed out: {method}")
            }
        }
    }
}

async fn fail_pending(pending: &PendingMap, message: String) {
    let mut pending = pending.lock().await;
    for (_, sender) in pending.drain() {
        let _ = sender.send(Err(anyhow::anyhow!(message.clone())));
    }
}

#[derive(Debug, Clone)]
pub struct ReconnectBackoff {
    current: Duration,
}

impl Default for ReconnectBackoff {
    fn default() -> Self {
        Self {
            current: Duration::from_millis(250),
        }
    }
}

impl ReconnectBackoff {
    pub fn next_delay(&mut self) -> Duration {
        let delay = self.current;
        self.current = self.current.saturating_mul(2).min(Duration::from_secs(30));
        delay
    }

    pub fn current(&self) -> Duration {
        self.current
    }

    pub fn reset(&mut self) {
        self.current = Duration::from_millis(250);
    }
}
