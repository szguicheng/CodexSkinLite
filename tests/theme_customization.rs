use std::collections::BTreeMap;

use codex_skin_lite::theme::{
    BackgroundCustomization, BackgroundFillMode, BackgroundImageCustomization,
    ComposerCustomization, PaletteCustomization, ShadowPreset, SurfaceCustomization, SurfacePart,
    ThemeCustomization, compile_customization_css,
};

#[test]
fn default_customization_has_safe_baseline_values() {
    let value = ThemeCustomization::default();

    assert_eq!(value.background.position_x, None);
    assert_eq!(value.background.position_y, None);
    assert_eq!(value.background.offset_x_px, 0);
    assert_eq!(value.background.offset_y_px, 0);
    assert_eq!(value.background.fill_mode, BackgroundFillMode::Cover);
    assert_eq!(value.background.opacity, 100);
    assert!(value.background.use_native_bottom_gradient);
    assert_eq!(value.background.image, None);
    assert_eq!(value.composer, ComposerCustomization::default());
    assert_eq!(value.colors, PaletteCustomization::default());
    assert!(value.surfaces.is_empty());
}

#[test]
fn bottom_gradient_preference_round_trips_and_old_themes_keep_native_gradient() {
    let old: ThemeCustomization = serde_json::from_str(r#"{"background":{"opacity":72}}"#).unwrap();
    assert!(old.background.use_native_bottom_gradient);
    let mut custom = old;
    custom.background.use_native_bottom_gradient = false;
    let encoded = serde_json::to_string(&custom).unwrap();
    assert!(encoded.contains("\"useNativeBottomGradient\":false"));
    assert_eq!(
        serde_json::from_str::<ThemeCustomization>(&encoded).unwrap(),
        custom
    );
}

#[test]
fn background_image_options_round_trip_through_the_customization_model() {
    let value = ThemeCustomization {
        background: BackgroundCustomization {
            image: Some(BackgroundImageCustomization {
                file_name: "custom-background.png".into(),
                source_path: None,
            }),
            offset_x_px: -24,
            offset_y_px: 18,
            fill_mode: BackgroundFillMode::Contain,
            opacity: 72,
            ..BackgroundCustomization::default()
        },
        ..ThemeCustomization::default()
    };

    let json = serde_json::to_value(&value).unwrap();

    assert_eq!(json["background"]["offsetXPx"], -24);
    assert_eq!(json["background"]["offsetYPx"], 18);
    assert_eq!(json["background"]["fillMode"], "contain");
    assert_eq!(json["background"]["opacity"], 72);
    assert_eq!(
        json["background"]["image"]["fileName"],
        "custom-background.png"
    );
}

#[test]
fn surface_parts_are_stable_and_serializable() {
    let mut surfaces = BTreeMap::new();
    surfaces.insert(
        SurfacePart::Composer,
        SurfaceCustomization {
            opacity: Some(88),
            blur_px: Some(12),
            radius_px: Some(20),
            shadow: Some(ShadowPreset::Soft),
        },
    );
    let value = ThemeCustomization {
        surfaces,
        ..ThemeCustomization::default()
    };

    let json = serde_json::to_value(&value).unwrap();
    assert_eq!(json["surfaces"]["composer"]["opacity"], 88);
    assert_eq!(json["surfaces"]["composer"]["shadow"], "soft");
}

#[test]
fn generated_css_uses_only_registered_surface_parts() {
    let value = ThemeCustomization {
        surfaces: BTreeMap::from([(
            SurfacePart::Thread,
            SurfaceCustomization {
                opacity: Some(90),
                blur_px: Some(8),
                radius_px: Some(16),
                shadow: Some(ShadowPreset::Strong),
            },
        )]),
        ..ThemeCustomization::default()
    };

    let css = compile_customization_css(&value).unwrap();

    assert!(css.contains("@layer dreamskin-community"));
    assert!(css.contains("[data-ds-part=\"thread\"]"));
    assert!(css.contains("backdrop-filter: blur(8px) !important"));
    assert!(!css.contains("body"));
}

#[test]
fn normalization_clamps_layout_values_and_trims_optional_colors() {
    let value = ThemeCustomization {
        background: BackgroundCustomization {
            position_x: Some(255),
            position_y: Some(255),
            ..BackgroundCustomization::default()
        },
        colors: PaletteCustomization {
            accent: Some("  #ABCDEF  ".into()),
            ..PaletteCustomization::default()
        },
        composer: ComposerCustomization {
            bottom_inset_px: 255,
            horizontal_inset_px: 255,
        },
        ..ThemeCustomization::default()
    }
    .normalized()
    .unwrap();

    assert_eq!(value.background.position_x, Some(100));
    assert_eq!(value.background.position_y, Some(100));
    assert_eq!(value.colors.accent.as_deref(), Some("#abcdef"));
    assert_eq!(value.composer.bottom_inset_px, 80);
    assert_eq!(value.composer.horizontal_inset_px, 120);
}
