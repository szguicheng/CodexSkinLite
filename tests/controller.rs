mod fixtures;

use std::sync::{Arc, Mutex};

use codex_skin_lite::controller::{AppCommand, Controller, RendererRuntime, RuntimeFuture, UiSink};
use codex_skin_lite::model::{AppSettings, AppSnapshot, ConnectionState};
use codex_skin_lite::paths::AppPaths;
use codex_skin_lite::renderer::RendererPayload;
use codex_skin_lite::settings::SettingsStore;
use codex_skin_lite::theme::ThemeStore;

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
async fn first_import_selects_theme_without_enabling_it() {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::for_test(dir.path());
    let settings = SettingsStore::new(paths.clone());
    let themes = ThemeStore::new(paths);
    let runtime = Arc::new(FakeRuntime {
        fail_apply: Mutex::new(false),
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
    });
    let sink = Arc::new(RecordingSink::default());

    Controller::new(settings.clone(), themes, runtime, sink).unwrap();

    let stored = settings.load().unwrap();
    assert_eq!(stored.active_theme_id.as_deref(), Some("eva-warm-cream"));
    assert!(!stored.theme_enabled);
}

struct ControllerEnvironment {
    _dir: tempfile::TempDir,
    settings: SettingsStore,
    controller: Controller,
    sink: Arc<RecordingSink>,
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
            ..AppSettings::default()
        })
        .unwrap();
    let runtime = Arc::new(FakeRuntime {
        fail_apply: Mutex::new(fail_apply),
    });
    let sink = Arc::new(RecordingSink::default());
    let controller = Controller::new(settings.clone(), themes, runtime, sink.clone()).unwrap();
    ControllerEnvironment {
        _dir: dir,
        settings,
        controller,
        sink,
    }
}

struct FakeRuntime {
    fail_apply: Mutex<bool>,
}

impl RendererRuntime for FakeRuntime {
    fn apply<'a>(&'a self, payload: &'a RendererPayload) -> RuntimeFuture<'a, u64> {
        Box::pin(async move {
            if std::mem::take(&mut *self.fail_apply.lock().unwrap()) {
                anyhow::bail!("injected failure");
            }
            Ok(payload.revision)
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
