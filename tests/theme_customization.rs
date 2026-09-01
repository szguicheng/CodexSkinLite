use std::collections::BTreeMap;

use codex_skin_lite::theme::{
    BackgroundCustomization, ComposerCustomization, PaletteCustomization, ShadowPreset,
    SurfaceCustomization, SurfacePart, ThemeCustomization, compile_customization_css,
};

#[test]
fn default_customization_has_safe_baseline_values() {
    let value = ThemeCustomization::default();

    assert_eq!(value.background.position_x, 50);
    assert_eq!(value.background.position_y, 50);
    assert_eq!(value.composer, ComposerCustomization::default());
    assert_eq!(value.colors, PaletteCustomization::default());
    assert!(value.surfaces.is_empty());
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
            position_x: 255,
            position_y: 255,
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

    assert_eq!(value.background.position_x, 100);
    assert_eq!(value.background.position_y, 100);
    assert_eq!(value.colors.accent.as_deref(), Some("#abcdef"));
    assert_eq!(value.composer.bottom_inset_px, 80);
    assert_eq!(value.composer.horizontal_inset_px, 120);
}
