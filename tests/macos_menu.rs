use codex_skin_lite::macos::MenuAction;

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
