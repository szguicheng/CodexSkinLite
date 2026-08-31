use std::net::IpAddr;
use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CdpTarget {
    pub id: String,
    pub title: String,
    pub url: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub web_socket_debugger_url: Option<String>,
}

pub async fn list_targets(debug_port: u16) -> anyhow::Result<Vec<CdpTarget>> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(Duration::from_millis(500))
        .timeout(Duration::from_secs(2))
        .build()?;
    let url = format!("http://127.0.0.1:{debug_port}/json/list");
    let targets = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<CdpTarget>>()
        .await?;
    for target in &targets {
        if let Some(websocket) = &target.web_socket_debugger_url {
            validate_websocket_url(websocket, debug_port)?;
        }
    }
    Ok(targets)
}

pub async fn endpoint_available(debug_port: u16) -> bool {
    list_targets(debug_port)
        .await
        .ok()
        .and_then(|targets| pick_primary_target(&targets).ok())
        .is_some()
}

pub fn pick_primary_target(targets: &[CdpTarget]) -> anyhow::Result<CdpTarget> {
    targets
        .iter()
        .filter(|target| is_injectable(target))
        .min_by_key(|target| target_rank(target))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no injectable Codex main target found"))
}

pub fn validate_websocket_url(value: &str, expected_port: u16) -> anyhow::Result<()> {
    let url = url::Url::parse(value)?;
    if url.scheme() != "ws" {
        anyhow::bail!("CDP WebSocket must use ws on loopback");
    }
    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("CDP WebSocket URL has no host"))?
        .parse::<IpAddr>()?;
    if !host.is_loopback() || url.port() != Some(expected_port) {
        anyhow::bail!("CDP WebSocket is not on the expected loopback port");
    }
    if !url.path().starts_with("/devtools/page/") {
        anyhow::bail!("CDP WebSocket is not a page target");
    }
    Ok(())
}

fn is_injectable(target: &CdpTarget) -> bool {
    if target.kind != "page" || target.web_socket_debugger_url.is_none() {
        return false;
    }
    let title = target.title.to_ascii_lowercase();
    let url = target.url.to_ascii_lowercase();
    if title.contains("quick chat")
        || url.contains("quick-chat")
        || url.contains("initialroute=%2favatar-overlay")
        || url.contains("initialroute=/avatar-overlay")
    {
        return false;
    }
    is_codex_url(&url)
        && (title.contains("codex")
            || (title == "chatgpt" && url.starts_with("app://-/index.html"))
            || url.starts_with("https://chatgpt.com/"))
}

fn is_codex_url(url: &str) -> bool {
    (url.starts_with("file://") && url.contains("index.html"))
        || url.starts_with("app://-/index.html")
        || url.starts_with("https://chatgpt.com/codex")
        || url.starts_with("https://chatgpt.com/c/")
}

fn target_rank(target: &CdpTarget) -> u8 {
    let title = target.title.to_ascii_lowercase();
    let url = target.url.to_ascii_lowercase();
    if title == "chatgpt" && url == "app://-/index.html" {
        0
    } else if title == "codex" && url.starts_with("file://") && url.contains("index.html") {
        1
    } else if title == "codex" || title == "chatgpt" {
        2
    } else {
        3
    }
}
