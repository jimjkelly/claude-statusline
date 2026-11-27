use std::process::Command;

#[test]
#[expect(
    clippy::unwrap_used,
    reason = "the binary under test is built by cargo"
)]
fn state_prints_json_with_supported_flag() {
    let out = Command::new(env!("CARGO_BIN_EXE_claude-statusline"))
        .arg("state")
        .output()
        .unwrap();
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let supported = json["supported"].as_bool().unwrap();
    assert_eq!(supported, cfg!(target_os = "macos"));
    if supported {
        let tier = json["tier"].as_str().unwrap();
        assert!(["full", "conserve", "minimal"].contains(&tier));
    }
}
