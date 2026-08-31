use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use crate::model::{AppSettings, AppSnapshot, ConnectionState, ThemeChoice};
use crate::renderer::RendererPayload;
use crate::settings::SettingsStore;
use crate::theme::{ThemeStore, ThemeSummary};

pub type RuntimeFuture<'a, T> = Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send + 'a>>;

pub trait RendererRuntime: Send + Sync {
    fn apply<'a>(&'a self, payload: &'a RendererPayload) -> RuntimeFuture<'a, u64>;
    fn open<'a>(&'a self, settings: &'a AppSettings) -> RuntimeFuture<'a, ConnectionState>;
    fn reconnect<'a>(&'a self, settings: &'a AppSettings) -> RuntimeFuture<'a, ConnectionState>;
    fn confirmed_restart<'a>(
        &'a self,
        settings: &'a AppSettings,
    ) -> RuntimeFuture<'a, ConnectionState>;
}

pub trait UiSink: Send + Sync {
    fn publish(&self, snapshot: AppSnapshot);
    fn report_error(&self, title: &str, message: &str);
}

#[derive(Debug)]
pub enum AppCommand {
    OpenCodex,
    ConfirmRestart,
    Reconnect,
    ImportTheme(PathBuf),
    ActivateTheme(String),
    SetThemeEnabled(bool),
    DeleteTheme(String),
    SetConversationCentered(bool),
    SetConversationWidth(u16),
    SetCodexPath(PathBuf),
    Shutdown,
}

pub struct Controller {
    settings_store: SettingsStore,
    theme_store: ThemeStore,
    runtime: Arc<dyn RendererRuntime>,
    sink: Arc<dyn UiSink>,
    settings: AppSettings,
    connection: ConnectionState,
    revision: u64,
}

impl Controller {
    pub fn new(
        settings_store: SettingsStore,
        theme_store: ThemeStore,
        runtime: Arc<dyn RendererRuntime>,
        sink: Arc<dyn UiSink>,
    ) -> anyhow::Result<Self> {
        let settings = settings_store.load()?;
        let controller = Self {
            settings_store,
            theme_store,
            runtime,
            sink,
            settings,
            connection: ConnectionState::Disconnected,
            revision: 0,
        };
        controller.publish();
        Ok(controller)
    }

    pub async fn handle(&mut self, command: AppCommand) -> anyhow::Result<()> {
        match command {
            AppCommand::OpenCodex => {
                self.connection = self.runtime.open(&self.settings).await?;
                self.publish();
            }
            AppCommand::ConfirmRestart => {
                self.connection = self.runtime.confirmed_restart(&self.settings).await?;
                self.publish();
            }
            AppCommand::Reconnect => {
                self.connection = self.runtime.reconnect(&self.settings).await?;
                self.publish();
            }
            AppCommand::ImportTheme(path) => {
                let bytes = std::fs::read(path)?;
                self.theme_store.import_zip_bytes(&bytes)?;
                self.publish();
            }
            AppCommand::ActivateTheme(id) => {
                self.theme_store.load_payload(&id)?;
                let mut next = self.settings.clone();
                next.active_theme_id = Some(id);
                next.theme_enabled = true;
                self.apply_candidate(next).await?;
            }
            AppCommand::SetThemeEnabled(enabled) => {
                if enabled && self.settings.active_theme_id.is_none() {
                    anyhow::bail!("select a theme before enabling presentation");
                }
                let mut next = self.settings.clone();
                next.theme_enabled = enabled;
                self.apply_candidate(next).await?;
            }
            AppCommand::DeleteTheme(id) => {
                self.theme_store
                    .delete(&id, self.settings.active_theme_id.as_deref())?;
                self.publish();
            }
            AppCommand::SetConversationCentered(enabled) => {
                let mut next = self.settings.clone();
                next.conversation_centered = enabled;
                self.apply_candidate(next).await?;
            }
            AppCommand::SetConversationWidth(width) => {
                let mut next = self.settings.clone();
                next.conversation_max_width = width.clamp(320, 4000);
                self.apply_candidate(next).await?;
            }
            AppCommand::SetCodexPath(path) => {
                self.settings.codex_app_path = path;
                self.settings_store.save(&self.settings)?;
                self.publish();
            }
            AppCommand::Shutdown => {}
        }
        Ok(())
    }

    fn payload(&self, settings: &AppSettings, revision: u64) -> anyhow::Result<RendererPayload> {
        let theme = if settings.theme_enabled {
            let id = settings
                .active_theme_id
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("enabled theme has no selected ID"))?;
            Some(self.theme_store.load_payload(id)?)
        } else {
            None
        };
        Ok(RendererPayload {
            revision,
            theme_enabled: settings.theme_enabled,
            theme,
            conversation_centered: settings.conversation_centered,
            conversation_max_width: settings.conversation_max_width,
        })
    }

    async fn apply_candidate(&mut self, candidate: AppSettings) -> anyhow::Result<()> {
        self.revision += 1;
        let candidate_payload = self.payload(&candidate, self.revision)?;
        let result = self.runtime.apply(&candidate_payload).await;
        if !matches!(result, Ok(revision) if revision == self.revision) {
            let error = match result {
                Ok(revision) => anyhow::anyhow!(
                    "renderer acknowledged stale revision {revision}; expected {}",
                    self.revision
                ),
                Err(error) => error,
            };
            self.revision += 1;
            if let Ok(previous) = self.payload(&self.settings, self.revision) {
                let _ = self.runtime.apply(&previous).await;
            }
            return Err(error);
        }
        self.settings_store.save(&candidate)?;
        self.settings = candidate;
        self.publish();
        Ok(())
    }

    fn publish(&self) {
        let themes = self.theme_store.list().unwrap_or_default();
        let active_theme_name = themes
            .iter()
            .into_iter()
            .find(|theme| Some(theme.id.as_str()) == self.settings.active_theme_id.as_deref())
            .map(|theme| theme.name.clone());
        self.sink.publish(AppSnapshot {
            settings: self.settings.clone(),
            connection: self.connection.clone(),
            active_theme_name,
            themes: themes
                .into_iter()
                .map(|theme: ThemeSummary| ThemeChoice {
                    id: theme.id,
                    name: theme.name,
                })
                .collect(),
        });
    }
}

pub struct NativeRuntime {
    launcher: crate::launcher::MacCodexLauncher,
    session: tokio::sync::Mutex<Option<crate::cdp::CdpSession>>,
}

impl Default for NativeRuntime {
    fn default() -> Self {
        Self {
            launcher: crate::launcher::MacCodexLauncher::default(),
            session: tokio::sync::Mutex::new(None),
        }
    }
}

impl NativeRuntime {
    async fn connect_once(&self, settings: &AppSettings) -> anyhow::Result<ConnectionState> {
        let targets = crate::cdp::list_targets(settings.debug_port).await?;
        let target = crate::cdp::pick_primary_target(&targets)?;
        let websocket = target
            .web_socket_debugger_url
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Codex target has no WebSocket URL"))?;
        crate::cdp::validate_websocket_url(websocket, settings.debug_port)?;
        let session = crate::cdp::CdpSession::connect(websocket).await?;
        session
            .install_bootstrap(crate::renderer::bootstrap_script())
            .await?;
        *self.session.lock().await = Some(session);
        Ok(ConnectionState::Connected)
    }

    async fn connect_after_launch(
        &self,
        settings: &AppSettings,
    ) -> anyhow::Result<ConnectionState> {
        let mut backoff = crate::cdp::ReconnectBackoff::default();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
        loop {
            match self.connect_once(settings).await {
                Ok(state) => return Ok(state),
                Err(error) if tokio::time::Instant::now() < deadline => {
                    tracing::debug!(%error, "waiting for Codex CDP endpoint");
                    tokio::time::sleep(backoff.next_delay()).await;
                }
                Err(error) => return Err(error),
            }
        }
    }
}

impl RendererRuntime for NativeRuntime {
    fn apply<'a>(&'a self, payload: &'a RendererPayload) -> RuntimeFuture<'a, u64> {
        Box::pin(async move {
            let session = self
                .session
                .lock()
                .await
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Codex is not connected"))?;
            let value = session
                .apply_payload(&serde_json::to_value(payload)?)
                .await?;
            value
                .get("revision")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| anyhow::anyhow!("renderer acknowledgment has no revision"))
        })
    }

    fn open<'a>(&'a self, settings: &'a AppSettings) -> RuntimeFuture<'a, ConnectionState> {
        Box::pin(async move {
            let cdp_available = crate::cdp::endpoint_available(settings.debug_port).await;
            match self
                .launcher
                .inspect(&settings.codex_app_path, cdp_available)?
            {
                crate::launcher::LaunchDecision::Attach => self.connect_once(settings).await,
                crate::launcher::LaunchDecision::Launch => {
                    self.launcher
                        .launch(&settings.codex_app_path, settings.debug_port)?;
                    self.connect_after_launch(settings).await
                }
                crate::launcher::LaunchDecision::RestartConfirmationRequired => {
                    Ok(ConnectionState::RestartRequired)
                }
            }
        })
    }

    fn reconnect<'a>(&'a self, settings: &'a AppSettings) -> RuntimeFuture<'a, ConnectionState> {
        Box::pin(async move { self.connect_once(settings).await })
    }

    fn confirmed_restart<'a>(
        &'a self,
        settings: &'a AppSettings,
    ) -> RuntimeFuture<'a, ConnectionState> {
        Box::pin(async move {
            self.launcher
                .terminate_after_confirmation(&settings.codex_app_path)?;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            self.launcher
                .launch(&settings.codex_app_path, settings.debug_port)?;
            self.connect_after_launch(settings).await
        })
    }
}

#[derive(Clone)]
pub struct ControllerHandle {
    sender: tokio::sync::mpsc::UnboundedSender<AppCommand>,
}

impl ControllerHandle {
    pub fn send(&self, command: AppCommand) -> anyhow::Result<()> {
        self.sender
            .send(command)
            .map_err(|_| anyhow::anyhow!("controller is stopped"))
    }
}

pub fn spawn(mut controller: Controller) -> ControllerHandle {
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let sink = controller.sink.clone();
    tokio::spawn(async move {
        while let Some(command) = receiver.recv().await {
            let shutdown = matches!(command, AppCommand::Shutdown);
            if let Err(error) = controller.handle(command).await {
                sink.report_error("CodexSkinLite", &error.to_string());
            }
            if shutdown {
                break;
            }
        }
    });
    ControllerHandle { sender }
}
