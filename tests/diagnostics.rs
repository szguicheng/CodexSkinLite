use codex_skin_lite::diagnostics::DiagnosticEvent;

#[test]
fn diagnostics_redact_conversation_and_credentials() {
    let event =
        DiagnosticEvent::cdp_error("Authorization: Bearer secret prompt=private conversation");

    let rendered = event.safe_message();

    assert!(!rendered.contains("secret"));
    assert!(!rendered.contains("private conversation"));
    assert!(rendered.contains("[REDACTED]"));
}
