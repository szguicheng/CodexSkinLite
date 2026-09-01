mod fixtures;

use std::sync::{Arc, Mutex};

use codex_skin_lite::controller::{AppCommand, Controller, RendererRuntime, RuntimeFuture, UiSink};
use codex_skin_lite::model::{AppSettings, AppSnapshot, ConnectionState};
use codex_skin_lite::paths::AppPaths;
use codex_skin_lite::renderer::RendererPayload;
use codex_skin_lite::settings::SettingsStore;
use codex_skin_lite::theme::{SurfacePart, ThemeStore};

#[tokio::test]
async fn failed_theme_apply_keeps_previous_active_theme() {
    let mut env = controller_environment(true);

    let result = env
        .controller
        .handle(AppCommand::ActivateTheme("blue-eyes".into()))
        .await;

    assert!(result.is_err());
    assert_eq!(
        env.settings.load().unwrap().active_theme_id.as_deref(),
        Some("eva-warm-cream")
    );
    assert!(matches!(
        env.sink
            .snapshots
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .connection,
        ConnectionState::CompatibilityWarning(_)
    ));
}

#[tokio::test]
async fn width_persists_only_after_renderer_verification() {
    let mut env = controller_environment(false);

    env.controller
        .handle(AppCommand::SetConversationWidth(880))
        .await
        .unwrap();

    assert_eq!(env.settings.load().unwrap().conversation_max_width, 880);
}

#[tokio::test]
async fn opening_codex_applies_saved_settings_to_renderer() {
    let mut env = controller_environment(false);

    env.controller.handle(AppCommand::OpenCodex).await.unwrap();

    let applied = env.applied_payloads.lock().unwrap();
    assert_eq!(applied.len(), 1);
    assert!(applied[0].theme_enabled);
    assert_eq!(
        applied[0].theme.as_ref().map(|theme| theme.id.as_str()),
        Some("eva-warm-cream")
    );
    assert!(applied[0].conversation_centered);
    assert_eq!(applied[0].conversation_max_width, 777);
}

#[test]
fn selected_theme_snapshot_contains_package_values_for_editor() {
    let env = controller_environment(false);
    let snapshot = env.sink.snapshots.lock().unwrap().last().unwrap().clone();

    assert_eq!(
        snapshot.active_theme_customization.background.position_x,
        Some(44)
    );
    assert_eq!(
        snapshot.active_theme_customization.background.position_y,
        Some(38)
    );
    assert_eq!(
        snapshot
            .active_theme_customization
            .colors
            .background
            .as_deref(),
        Some("#fffaf0")
    );
    assert_eq!(
        snapshot.active_theme_customization.colors.panel.as_deref(),
        Some("#fff8e8")
    );
    assert_eq!(
        snapshot
            .active_theme_customization
            .surfaces
            .get(&SurfacePart::Composer)
            .and_then(|surface| surface.radius_px),
        Some(18)
    );
}

#[tokio::test]
async fn reconnect_resends_saved_settings_after_renderer_revision_survives_restart() {
    let mut env = controller_environment(false);
    *env.renderer_revision.lock().unwrap() = 50;

    env.controller.handle(AppCommand::Reconnect).await.unwrap();

    assert_eq!(*env.renderer_revision.lock().unwrap(), 51);
    let applied = env.applied_payloads.lock().unwrap();
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].revision, 51);
    assert!(applied[0].theme_enabled);
    assert!(applied[0].conversation_centered);
    assert_eq!(applied[0].conversation_max_width, 777);
}

#[tokio::test]
async fn every_successful_connection_path_applies_saved_settings() {
    let mut env = controller_environment(false);

    for command in [
        AppCommand::OpenCodex,
        AppCommand::Reconnect,
        AppCommand::ConfirmRestart,
    ] {
        env.controller.handle(command).await.unwrap();
    }

    let applied = env.applied_payloads.lock().unwrap();
    assert_eq!(applied.len(), 3);
    assert!(applied.iter().all(|payload| {
        payload.theme_enabled
            && payload.conversation_centered
            && payload.conversation_max_width == 777
            && payload.theme.as_ref().map(|theme| theme.id.as_str()) == Some("eva-warm-cream")
    }));
}

#[tokio::test]
async fn preview_customization_applies_without_writing_the_file() {
    let mut env = controller_environment(false);
    let candidate = codex_skin_lite::theme::ThemeCustomization {
        background: codex_skin_lite::theme::BackgroundCustomization {
            position_x: Some(12),
            position_y: Some(84),
        },
        ..Default::default()
    };

    env.controller.handle(AppCommand::OpenCodex).await.unwrap();
    env.controller
        .handle(AppCommand::PreviewThemeCustomization(candidate.clone()))
        .await
        .unwrap();

    assert_eq!(
        env.themes.load_customization("eva-warm-cream").unwrap(),
        codex_skin_lite::theme::ThemeCustomization::default()
    );
    assert_eq!(
        env.applied_payloads
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .theme
            .as_ref()
            .unwrap()
            .customization,
        candidate
    );
}

#[tokio::test]
async fn save_customization_persists_and_is_sent_on_the_next_connection() {
    let mut env = controller_environment(false);
    let candidate = codex_skin_lite::theme::ThemeCustomization {
        composer: codex_skin_lite::theme::ComposerCustomization {
            bottom_inset_px: 16,
            horizontal_inset_px: 24,
        },
        ..Default::default()
    };

    env.controller
        .handle(AppCommand::SaveThemeCustomization(candidate.clone()))
        .await
        .unwrap();
    assert_eq!(
        env.themes.load_customization("eva-warm-cream").unwrap(),
        candidate
    );

    env.controller.handle(AppCommand::Reconnect).await.unwrap();
    assert_eq!(
        env.applied_payloads
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .theme
            .as_ref()
            .unwrap()
            .customization,
        candidate
    );
}

#[tokio::test]
async fn first_import_selects_theme_without_enabling_it() {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::for_test(dir.path());
    let settings = SettingsStore::new(paths.clone());
    let themes = ThemeStore::new(paths);
    let runtime = Arc::new(FakeRuntime {
        fail_apply: Mutex::new(false),
        applied_payloads: Arc::new(Mutex::new(Vec::new())),
        renderer_revision: Arc::new(Mutex::new(0)),
    });
    let sink = Arc::new(RecordingSink::default());
    let mut controller = Controller::new(settings.clone(), themes, runtime, sink).unwrap();
    let zip_path = dir.path().join("eva.zip");
    std::fs::write(&zip_path, fixtures::valid_theme_zip()).unwrap();

    controller
        .handle(AppCommand::ImportTheme(zip_path))
        .await
        .unwrap();

    let stored = settings.load().unwrap();
    assert_eq!(stored.active_theme_id.as_deref(), Some("eva-warm-cream"));
    assert!(!stored.theme_enabled);
}

#[tokio::test]
async fn selecting_no_theme_clears_selection_and_disables_theme() {
    let mut env = controller_environment(false);

    env.controller
        .handle(AppCommand::ActivateTheme(String::new()))
        .await
        .unwrap();

    let stored = env.settings.load().unwrap();
    assert_eq!(stored.active_theme_id, None);
    assert!(!stored.theme_enabled);
}

#[test]
fn startup_selects_the_only_imported_theme_when_settings_are_empty() {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::for_test(dir.path());
    let settings = SettingsStore::new(paths.clone());
    let themes = ThemeStore::new(paths);
    themes
        .import_zip_bytes(&fixtures::valid_theme_zip())
        .unwrap();
    let runtime = Arc::new(FakeRuntime {
        fail_apply: Mutex::new(false),
        applied_payloads: Arc::new(Mutex::new(Vec::new())),
        renderer_revision: Arc::new(Mutex::new(0)),
    });
    let sink = Arc::new(RecordingSink::default());

    Controller::new(settings.clone(), themes, runtime, sink).unwrap();

    let stored = settings.load().unwrap();
    assert_eq!(stored.active_theme_id, None);
    assert!(!stored.theme_enabled);
}

struct ControllerEnvironment {
    _dir: tempfile::TempDir,
    settings: SettingsStore,
    themes: ThemeStore,
    controller: Controller,
    sink: Arc<RecordingSink>,
    applied_payloads: Arc<Mutex<Vec<RendererPayload>>>,
    renderer_revision: Arc<Mutex<u64>>,
}

fn controller_environment(fail_apply: bool) -> ControllerEnvironment {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::for_test(dir.path());
    let settings = SettingsStore::new(paths.clone());
    let themes = ThemeStore::new(paths);
    themes
        .import_zip_bytes(&fixtures::valid_theme_zip())
        .unwrap();
    themes
        .import_zip_bytes(&fixtures::theme_zip(fixtures::ThemeZipOptions {
            theme_id: "blue-eyes".into(),
            ..fixtures::ThemeZipOptions::valid()
        }))
        .unwrap();
    settings
        .save(&AppSettings {
            theme_enabled: true,
            active_theme_id: Some("eva-warm-cream".into()),
            conversation_centered: true,
            conversation_max_width: 777,
            ..AppSettings::default()
        })
        .unwrap();
    let applied_payloads = Arc::new(Mutex::new(Vec::new()));
    let renderer_revision = Arc::new(Mutex::new(0));
    let runtime = Arc::new(FakeRuntime {
        fail_apply: Mutex::new(fail_apply),
        applied_payloads: applied_payloads.clone(),
        renderer_revision: renderer_revision.clone(),
    });
    let sink = Arc::new(RecordingSink::default());
    let controller =
        Controller::new(settings.clone(), themes.clone(), runtime, sink.clone()).unwrap();
    ControllerEnvironment {
        _dir: dir,
        settings,
        themes,
        controller,
        sink,
        applied_payloads,
        renderer_revision,
    }
}

struct FakeRuntime {
    fail_apply: Mutex<bool>,
    applied_payloads: Arc<Mutex<Vec<RendererPayload>>>,
    renderer_revision: Arc<Mutex<u64>>,
}

impl RendererRuntime for FakeRuntime {
    fn apply<'a>(&'a self, payload: &'a RendererPayload) -> RuntimeFuture<'a, u64> {
        Box::pin(async move {
            if std::mem::take(&mut *self.fail_apply.lock().unwrap()) {
                anyhow::bail!("injected failure");
            }
            let mut renderer_revision = self.renderer_revision.lock().unwrap();
            if payload.revision <= *renderer_revision {
                return Ok(*renderer_revision);
            }
            *renderer_revision = payload.revision;
            self.applied_payloads.lock().unwrap().push(payload.clone());
            Ok(*renderer_revision)
        })
    }

    fn open<'a>(&'a self, _settings: &'a AppSettings) -> RuntimeFuture<'a, ConnectionState> {
        Box::pin(async { Ok(ConnectionState::Connected) })
    }

    fn reconnect<'a>(&'a self, _settings: &'a AppSettings) -> RuntimeFuture<'a, ConnectionState> {
        Box::pin(async { Ok(ConnectionState::Connected) })
    }

    fn confirmed_restart<'a>(
        &'a self,
        _settings: &'a AppSettings,
    ) -> RuntimeFuture<'a, ConnectionState> {
        Box::pin(async { Ok(ConnectionState::Connected) })
    }
}

#[derive(Default)]
struct RecordingSink {
    snapshots: Mutex<Vec<AppSnapshot>>,
}

impl UiSink for RecordingSink {
    fn publish(&self, snapshot: AppSnapshot) {
        self.snapshots.lock().unwrap().push(snapshot);
    }

    fn report_error(&self, _title: &str, _message: &str) {}
}
