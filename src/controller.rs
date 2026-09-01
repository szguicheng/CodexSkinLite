use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use crate::model::{AppSettings, AppSnapshot, ConnectionState, ThemeChoice};
use crate::renderer::RendererPayload;
use crate::settings::SettingsStore;
use crate::theme::{ThemeCustomization, ThemeStore, ThemeSummary};

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
    PreviewThemeCustomization(ThemeCustomization),
    SaveThemeCustomization(ThemeCustomization),
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
    preview_customization: Option<ThemeCustomization>,
}

impl Controller {
    pub fn new(
        settings_store: SettingsStore,
        theme_store: ThemeStore,
        runtime: Arc<dyn RendererRuntime>,
        sink: Arc<dyn UiSink>,
    ) -> anyhow::Result<Self> {
        let mut settings = settings_store.load()?;
        if settings.active_theme_id.is_none() {
            let themes = theme_store.list()?;
            if themes.len() == 1 {
                settings.active_theme_id = Some(themes[0].id.clone());
                settings_store.save(&settings)?;
            }
        }
        let controller = Self {
            settings_store,
            theme_store,
            runtime,
            sink,
            settings,
            connection: ConnectionState::Disconnected,
            revision: 0,
            preview_customization: None,
        };
        controller.publish();
        Ok(controller)
    }

    pub async fn handle(&mut self, command: AppCommand) -> anyhow::Result<()> {
        let result = self.handle_inner(command).await;
        if let Err(error) = &result {
            self.connection = ConnectionState::CompatibilityWarning(error.to_string());
            self.publish();
            self.sink.report_error("CodexSkinLite", &error.to_string());
        }
        result
    }

    async fn handle_inner(&mut self, command: AppCommand) -> anyhow::Result<()> {
        match command {
            AppCommand::OpenCodex => {
                self.preview_customization = None;
                self.connection = self.runtime.open(&self.settings).await?;
                if matches!(self.connection, ConnectionState::Connected) {
                    self.apply_saved_settings().await?;
                }
                self.publish();
            }
            AppCommand::ConfirmRestart => {
                self.preview_customization = None;
                self.connection = self.runtime.confirmed_restart(&self.settings).await?;
                if matches!(self.connection, ConnectionState::Connected) {
                    self.apply_saved_settings().await?;
                }
                self.publish();
            }
            AppCommand::Reconnect => {
                self.preview_customization = None;
                self.connection = self.runtime.reconnect(&self.settings).await?;
                if matches!(self.connection, ConnectionState::Connected) {
                    self.apply_saved_settings().await?;
                }
                self.publish();
            }
            AppCommand::ImportTheme(path) => {
                let bytes = std::fs::read(path)?;
                let imported = self.theme_store.import_zip_bytes(&bytes)?;
                if self.settings.active_theme_id.is_none() {
                    self.settings.active_theme_id = Some(imported.id);
                    self.settings_store.save(&self.settings)?;
                }
                self.publish();
            }
            AppCommand::ActivateTheme(id) => {
                self.theme_store.load_payload(&id)?;
                self.preview_customization = None;
                let mut next = self.settings.clone();
                next.active_theme_id = Some(id);
                next.theme_enabled = true;
                self.apply_candidate(next).await?;
            }
            AppCommand::SetThemeEnabled(enabled) => {
                if enabled && self.settings.active_theme_id.is_none() {
                    anyhow::bail!("select a theme before enabling presentation");
                }
                self.preview_customization = None;
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
                self.preview_customization = None;
                let mut next = self.settings.clone();
                next.conversation_centered = enabled;
                self.apply_candidate(next).await?;
            }
            AppCommand::SetConversationWidth(width) => {
                self.preview_customization = None;
                let mut next = self.settings.clone();
                next.conversation_max_width = width.clamp(320, 4000);
                self.apply_candidate(next).await?;
            }
            AppCommand::SetCodexPath(path) => {
                self.settings.codex_app_path = path;
                self.settings_store.save(&self.settings)?;
                self.publish();
            }
            AppCommand::PreviewThemeCustomization(customization) => {
                self.preview_theme_customization(customization).await?;
            }
            AppCommand::SaveThemeCustomization(customization) => {
                self.save_theme_customization(customization).await?;
            }
            AppCommand::Shutdown => {}
        }
        Ok(())
    }

    fn payload(&self, settings: &AppSettings, revision: u64) -> anyhow::Result<RendererPayload> {
        self.payload_with_customization(settings, revision, None)
    }

    fn payload_with_customization(
        &self,
        settings: &AppSettings,
        revision: u64,
        customization: Option<&ThemeCustomization>,
    ) -> anyhow::Result<RendererPayload> {
        let theme = if settings.theme_enabled {
            let id = settings
                .active_theme_id
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("enabled theme has no selected ID"))?;
            Some(match customization {
                Some(customization) => self
                    .theme_store
                    .load_payload_with_customization(id, customization)?,
                None => self.theme_store.load_payload(id)?,
            })
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

    async fn preview_theme_customization(
        &mut self,
        customization: ThemeCustomization,
    ) -> anyhow::Result<()> {
        if !matches!(self.connection, ConnectionState::Connected) {
            anyhow::bail!("Codex is not connected; preview requires an active connection");
        }
        if self.settings.active_theme_id.is_none() {
            anyhow::bail!("select a theme before previewing customization");
        }
        let customization = customization.normalized()?;
        let mut preview_settings = self.settings.clone();
        preview_settings.theme_enabled = true;
        self.revision += 1;
        let payload = self.payload_with_customization(
            &preview_settings,
            self.revision,
            Some(&customization),
        )?;
        if let Err(error) = self.apply_renderer_payload(&payload).await {
            self.restore_saved_payload().await;
            return Err(error);
        }
        self.preview_customization = Some(customization);
        self.publish();
        Ok(())
    }

    async fn save_theme_customization(
        &mut self,
        customization: ThemeCustomization,
    ) -> anyhow::Result<()> {
        let id = self
            .settings
            .active_theme_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("select a theme before saving customization"))?
            .to_string();
        let customization = customization.normalized()?;
        let previous = self.theme_store.load_customization(&id)?;
        let apply_now =
            self.settings.theme_enabled && matches!(self.connection, ConnectionState::Connected);
        if apply_now {
            self.revision += 1;
            let payload = self.payload_with_customization(
                &self.settings,
                self.revision,
                Some(&customization),
            )?;
            if let Err(error) = self.apply_renderer_payload(&payload).await {
                self.restore_saved_payload().await;
                return Err(error);
            }
        }
        if let Err(error) = self.theme_store.save_customization(&id, &customization) {
            if apply_now {
                self.revision += 1;
                if let Ok(payload) =
                    self.payload_with_customization(&self.settings, self.revision, Some(&previous))
                {
                    let _ = self.apply_renderer_payload(&payload).await;
                }
            }
            return Err(error.into());
        }
        self.preview_customization = None;
        self.publish();
        Ok(())
    }

    async fn apply_renderer_payload(&self, payload: &RendererPayload) -> anyhow::Result<()> {
        let acknowledged_revision = self.runtime.apply(payload).await?;
        if acknowledged_revision != payload.revision {
            anyhow::bail!(
                "renderer acknowledged stale revision {acknowledged_revision}; expected {}",
                payload.revision
            );
        }
        Ok(())
    }

    async fn restore_saved_payload(&mut self) {
        self.revision += 1;
        if let Ok(previous) = self.payload(&self.settings, self.revision) {
            let _ = self.apply_renderer_payload(&previous).await;
        }
    }

    async fn apply_saved_settings(&mut self) -> anyhow::Result<()> {
        let mut expected_revision = self.revision.saturating_add(1);
        for attempt in 0..2 {
            self.revision = expected_revision;
            let payload = self.payload(&self.settings, expected_revision)?;
            let acknowledged_revision = self.runtime.apply(&payload).await?;
            if acknowledged_revision == expected_revision {
                return Ok(());
            }
            if acknowledged_revision > expected_revision && attempt == 0 {
                expected_revision = acknowledged_revision.saturating_add(1);
                continue;
            }
            anyhow::bail!(
                "renderer acknowledged stale revision {acknowledged_revision}; expected {expected_revision}"
            );
        }
        unreachable!("saved settings application has a bounded retry")
    }

    async fn apply_candidate(&mut self, candidate: AppSettings) -> anyhow::Result<()> {
        self.revision += 1;
        let candidate_payload = self.payload(&candidate, self.revision)?;
        if let Err(error) = self.apply_renderer_payload(&candidate_payload).await {
            self.restore_saved_payload().await;
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
        let active_theme_customization = self
            .settings
            .active_theme_id
            .as_deref()
            .and_then(|id| self.theme_store.load_customization(id).ok())
            .unwrap_or_default();
        self.sink.publish(AppSnapshot {
            settings: self.settings.clone(),
            connection: self.connection.clone(),
            active_theme_name,
            active_theme_customization,
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
    tokio::spawn(async move {
        while let Some(command) = receiver.recv().await {
            let shutdown = matches!(command, AppCommand::Shutdown);
            let _ = controller.handle(command).await;
            if shutdown {
                break;
            }
        }
    });
    ControllerHandle { sender }
}
