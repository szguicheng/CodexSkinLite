mod fixtures;

use codex_skin_lite::theme::ThemeError;

#[test]
fn import_publishes_complete_theme_atomically() {
    let env = fixtures::theme_environment();

    let summary = env
        .store
        .import_zip_bytes(&fixtures::valid_theme_zip())
        .unwrap();

    let dir = env.paths.themes_dir().join(&summary.id);
    assert!(dir.join("manifest.json").is_file());
    assert!(dir.join("compiled.css").is_file());
    assert!(!env.paths.themes_dir().join(".importing").exists());
}

#[test]
fn active_theme_cannot_be_deleted() {
    let env = fixtures::theme_environment_with_active("eva-warm-cream");

    assert!(matches!(
        env.store.delete("eva-warm-cream", Some("eva-warm-cream")),
        Err(ThemeError::ActiveTheme)
    ));
}

#[test]
fn load_payload_rejects_tampered_stored_manifest() {
    let env = fixtures::theme_environment();
    env.store
        .import_zip_bytes(&fixtures::valid_theme_zip())
        .unwrap();
    std::fs::write(
        env.paths.themes_dir().join("eva-warm-cream/manifest.json"),
        b"{}",
    )
    .unwrap();

    assert!(matches!(
        env.store.load_payload("eva-warm-cream"),
        Err(ThemeError::InvalidStoredTheme(_))
    ));
}
