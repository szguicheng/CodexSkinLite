mod fixtures;

use codex_skin_lite::theme::{ThemeError, validate_package};

#[test]
fn accepts_minimal_macos_package() {
    let bytes = fixtures::valid_theme_zip();

    let package = validate_package(&bytes).unwrap();

    assert_eq!(package.manifest.theme_id, "eva-warm-cream");
    assert_eq!(package.image_name, "background.png");
}

#[test]
fn rejects_parent_traversal_before_extracting() {
    let bytes = fixtures::theme_zip(fixtures::ThemeZipOptions {
        extra_entry: Some(("../escape.txt".into(), b"bad".to_vec())),
        ..fixtures::ThemeZipOptions::valid()
    });

    assert!(matches!(
        validate_package(&bytes),
        Err(ThemeError::UnsafePath(_))
    ));
}

#[test]
fn rejects_wrong_platform_and_payload_hash() {
    let wrong_platform = fixtures::theme_zip(fixtures::ThemeZipOptions {
        platform: "windows".into(),
        ..fixtures::ThemeZipOptions::valid()
    });
    assert!(matches!(
        validate_package(&wrong_platform),
        Err(ThemeError::InvalidManifest(_))
    ));

    let wrong_hash = fixtures::theme_zip(fixtures::ThemeZipOptions {
        image_hash: Some("0".repeat(64)),
        ..fixtures::ThemeZipOptions::valid()
    });
    assert!(matches!(
        validate_package(&wrong_hash),
        Err(ThemeError::InvalidManifest(_))
    ));
}

#[test]
fn rejects_image_magic_mismatch() {
    let bytes = fixtures::theme_zip(fixtures::ThemeZipOptions {
        image_bytes: b"not a png".to_vec(),
        ..fixtures::ThemeZipOptions::valid()
    });
    assert!(matches!(
        validate_package(&bytes),
        Err(ThemeError::InvalidImage(_))
    ));
}

#[test]
fn rejects_manifest_without_required_safe_css_capability() {
    let bytes = fixtures::theme_zip(fixtures::ThemeZipOptions {
        capabilities: vec!["background".into(), "tokens".into()],
        ..fixtures::ThemeZipOptions::valid()
    });
    assert!(matches!(
        validate_package(&bytes),
        Err(ThemeError::InvalidManifest(_))
    ));
}

#[test]
fn rejects_manifest_media_type_that_does_not_match_the_image() {
    let bytes = fixtures::theme_zip(fixtures::ThemeZipOptions {
        image_media_type: "text/plain".into(),
        ..fixtures::ThemeZipOptions::valid()
    });
    assert!(matches!(
        validate_package(&bytes),
        Err(ThemeError::InvalidManifest(_))
    ));
}
