use codex_skin_lite::theme::compile_safe_css;

#[test]
fn compiles_registered_part_and_adds_trusted_priority() {
    let result = compile_safe_css(
        r#"[data-ds-part="main"] { background-color: rgba(255,250,240,0.65); backdrop-filter: blur(8px); }"#,
    )
    .unwrap();

    assert!(result.contains("@layer dreamskin-community"));
    assert!(result.contains("backdrop-filter: blur(8px) !important"));
}

#[test]
fn rejects_arbitrary_selectors_and_remote_urls() {
    assert!(compile_safe_css("body div { color: red; }").is_err());
    assert!(
        compile_safe_css(
            r#"[data-ds-part="main"] { background-image: url(https://example.com/x); }"#
        )
        .is_err()
    );
}

#[test]
fn compiles_the_current_eva_and_blue_eyes_theme_css() {
    for css in [
        include_str!("data/eva-warm.css"),
        include_str!("data/blue-eyes.css"),
    ] {
        let compiled = compile_safe_css(css).unwrap();
        assert!(compiled.starts_with("@layer dreamskin-community"));
        assert!(!compiled.contains("url("));
    }
}
