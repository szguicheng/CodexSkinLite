use codex_skin_lite::model::AppSettings;
use codex_skin_lite::paths::AppPaths;
use codex_skin_lite::settings::SettingsStore;

#[test]
fn invalid_width_and_port_are_normalized_without_losing_other_fields() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::new(AppPaths::for_test(dir.path()));
    std::fs::create_dir_all(store.paths().root()).unwrap();
    std::fs::write(
        store.paths().settings_file(),
        r#"{
          "debugPort": 0,
          "themeEnabled": true,
          "activeThemeId": " eva ",
          "conversationCentered": true,
          "conversationMaxWidth": 9000,
          "codexAppPath": "/Applications/Codex.app"
        }"#,
    )
    .unwrap();

    let value = store.load().unwrap();

    assert_eq!(value.debug_port, 9222);
    assert_eq!(value.conversation_max_width, 4000);
    assert_eq!(value.active_theme_id.as_deref(), Some("eva"));
    assert!(value.theme_enabled);
}

#[test]
fn save_replaces_settings_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let store = SettingsStore::new(AppPaths::for_test(dir.path()));

    store.save(&AppSettings::default()).unwrap();

    assert!(!store.paths().settings_temp_file().exists());
    assert!(store.paths().settings_file().is_file());
    let round_trip = store.load().unwrap();
    assert_eq!(round_trip, AppSettings::default());
}
