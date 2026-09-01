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

#[test]
fn customization_round_trips_without_changing_the_theme_package() {
    let env = fixtures::theme_environment();
    env.store
        .import_zip_bytes(&fixtures::valid_theme_zip())
        .unwrap();
    let customization = codex_skin_lite::theme::ThemeCustomization {
        background: codex_skin_lite::theme::BackgroundCustomization {
            position_x: 18,
            position_y: 72,
        },
        ..Default::default()
    };

    env.store
        .save_customization("eva-warm-cream", &customization)
        .unwrap();
    let loaded = env.store.load_customization("eva-warm-cream").unwrap();

    assert_eq!(loaded, customization);
    assert!(
        env.paths
            .themes_dir()
            .join("eva-warm-cream/theme.css")
            .is_file()
    );
}

#[test]
fn corrupt_customization_falls_back_to_defaults() {
    let env = fixtures::theme_environment();
    env.store
        .import_zip_bytes(&fixtures::valid_theme_zip())
        .unwrap();
    std::fs::write(
        env.paths
            .themes_dir()
            .join("eva-warm-cream/customization.json"),
        b"{not-json}",
    )
    .unwrap();

    assert_eq!(
        env.store.load_customization("eva-warm-cream").unwrap(),
        codex_skin_lite::theme::ThemeCustomization::default()
    );
}
