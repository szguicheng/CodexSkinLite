use std::path::Path;

use codex_skin_lite::launcher::{
    LaunchDecision, build_open_command, decide_launch, validate_codex_bundle,
};

#[test]
fn running_without_cdp_requires_confirmation() {
    assert_eq!(
        decide_launch(true, false),
        LaunchDecision::RestartConfirmationRequired
    );
}

#[test]
fn command_binds_debugging_to_loopback() {
    let cmd = build_open_command(Path::new("/Applications/Codex.app"), 9222);
    assert_eq!(
        &cmd[..5],
        &[
            "open".to_string(),
            "-W".to_string(),
            "-a".to_string(),
            "/Applications/Codex.app".to_string(),
            "--args".to_string(),
        ]
    );
    assert!(cmd.contains(&"--remote-debugging-address=127.0.0.1".to_string()));
    assert!(cmd.contains(&"--remote-debugging-port=9222".to_string()));
}

#[test]
fn stopped_and_already_debuggable_states_do_not_request_restart() {
    assert_eq!(decide_launch(false, false), LaunchDecision::Launch);
    assert_eq!(decide_launch(true, true), LaunchDecision::Attach);
}

#[test]
fn accepts_current_chatgpt_named_codex_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("ChatGPT.app");
    std::fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
    std::fs::write(app.join("Contents/MacOS/ChatGPT"), b"fixture").unwrap();

    validate_codex_bundle(&app).unwrap();
}
