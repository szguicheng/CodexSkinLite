use std::sync::{Arc, Mutex};

use codex_skin_lite::controller::UiSink;
use codex_skin_lite::macos::{AppKitSink, AppKitState, MenuAction};
use codex_skin_lite::model::{AppSettings, AppSnapshot, ConnectionState};
use codex_skin_lite::theme::ThemeCustomization;

#[test]
fn menu_model_exposes_only_approved_actions() {
    assert_eq!(
        MenuAction::ALL,
        [
            MenuAction::OpenCodex,
            MenuAction::Reconnect,
            MenuAction::OpenSettings,
            MenuAction::Quit,
        ]
    );
}

#[test]
fn publishing_snapshot_notifies_registered_ui_refresher() {
    let state = Arc::new(AppKitState::default());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_for_callback = seen.clone();
    state.set_refresher(Arc::new(move |snapshot| {
        seen_for_callback
            .lock()
            .unwrap()
            .push(snapshot.connection.clone());
    }));
    let sink = AppKitSink::new(state);

    sink.publish(AppSnapshot {
        settings: AppSettings::default(),
        connection: ConnectionState::Connected,
        active_theme_name: None,
        active_theme_customization: ThemeCustomization::default(),
        themes: Vec::new(),
    });

    assert_eq!(*seen.lock().unwrap(), vec![ConnectionState::Connected]);
}
