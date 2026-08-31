use codex_skin_lite::model::AppSettings;

#[test]
fn defaults_match_product_contract() {
    let value = AppSettings::default();
    assert_eq!(value.debug_port, 9222);
    assert!(!value.theme_enabled);
    assert_eq!(value.active_theme_id, None);
    assert!(!value.conversation_centered);
    assert_eq!(value.conversation_max_width, 900);
}
