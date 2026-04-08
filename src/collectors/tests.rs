use super::commands::{recent_commands, sanitize_command};

#[test]
fn redacts_inline_secret_assignments() {
    assert_eq!(
        sanitize_command("export GEMINI_API_KEY=super-secret-value"),
        "export GEMINI_API_KEY=[REDACTED]"
    );
}

#[test]
fn redacts_setx_secret_commands() {
    assert_eq!(
        sanitize_command(r#"setx GEMINI_API_KEY "abc123""#),
        "setx GEMINI_API_KEY [REDACTED]"
    );
}

#[test]
fn recent_commands_does_not_panic_without_history() {
    let commands = recent_commands(3);
    assert!(commands.len() <= 3);
}
