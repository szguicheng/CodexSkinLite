mod fixtures;

use codex_skin_lite::theme::ThemeError;
use codex_skin_lite::theme::{BackgroundImageCustomization, ThemeCustomization};

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
            position_x: Some(18),
            position_y: Some(72),
            ..Default::default()
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
fn editor_reopens_saved_gradient_and_other_customization_fields() {
    use codex_skin_lite::theme::{BackgroundFillMode, ShadowPreset, SurfacePart};
    let env = fixtures::theme_environment();
    env.store
        .import_zip_bytes(&fixtures::valid_theme_zip())
        .unwrap();
    let mut draft = env
        .store
        .load_editor_customization("eva-warm-cream")
        .unwrap();
    draft.background.position_x = Some(12);
    draft.background.position_y = Some(78);
    draft.background.offset_x_px = -24;
    draft.background.offset_y_px = 37;
    draft.background.fill_mode = BackgroundFillMode::Contain;
    draft.background.opacity = 65;
    draft.colors.background = Some("#123456".into());
    draft.colors.panel = Some("#234567".into());
    draft.colors.accent = Some("#345678".into());
    draft.colors.text = Some("#456789".into());
    draft.colors.line = Some("#567890".into());
    for part in SurfacePart::ALL {
        let surface = draft.surfaces.entry(part).or_default();
        surface.opacity = Some(85);
        surface.blur_px = Some(7);
        surface.radius_px = Some(12);
        surface.shadow = Some(ShadowPreset::Soft);
    }
    draft.composer.bottom_inset_px = 17;
    draft.composer.horizontal_inset_px = 23;
    for (top, bottom) in [(false, true), (true, false), (false, false), (true, true)] {
        draft.background.use_native_top_gradient = top;
        draft.background.use_native_bottom_gradient = bottom;
        env.store
            .save_customization("eva-warm-cream", &draft)
            .unwrap();
        assert_eq!(
            env.store.load_customization("eva-warm-cream").unwrap(),
            draft
        );
        assert_eq!(
            env.store
                .load_editor_customization("eva-warm-cream")
                .unwrap(),
            draft
        );
    }
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

#[test]
fn selected_background_image_is_copied_without_replacing_the_theme_image() {
    let env = fixtures::theme_environment();
    env.store
        .import_zip_bytes(&fixtures::valid_theme_zip())
        .unwrap();
    let source = env._dir.path().join("selected.png");
    std::fs::write(&source, fixtures::ThemeZipOptions::valid().image_bytes).unwrap();

    let customization = ThemeCustomization {
        background: codex_skin_lite::theme::BackgroundCustomization {
            image: Some(BackgroundImageCustomization {
                file_name: "selected.png".into(),
                source_path: Some(source),
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    env.store
        .save_customization("eva-warm-cream", &customization)
        .unwrap();

    let theme_dir = env.paths.themes_dir().join("eva-warm-cream");
    assert!(theme_dir.join("background.png").is_file());
    assert!(theme_dir.join("custom-background.png").is_file());
    assert_eq!(
        env.store
            .load_customization("eva-warm-cream")
            .unwrap()
            .background
            .image
            .as_ref()
            .unwrap()
            .file_name,
        "custom-background.png"
    );
}
