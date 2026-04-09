use dirs::home_dir;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

pub fn recent_commands(limit: usize) -> Vec<String> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };

    let mut combined = Vec::new();
    for history_file in history_files(&home) {
        let Ok(content) = fs::read_to_string(history_file) else {
            continue;
        };

        combined.extend(content.lines().rev().map(parse_shell_history_line));
    }

    dedupe_and_trim(combined, limit)
}

pub(crate) fn sanitize_command(command: &str) -> String {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let redacted = redact_assignment(trimmed);
    redact_setx(&redacted)
}

fn history_files(home: &PathBuf) -> Vec<PathBuf> {
    vec![
        home.join(".bash_history"),
        home.join(".zsh_history"),
        home.join(".local/share/fish/fish_history"),
        home.join(
            r"AppData\Roaming\Microsoft\Windows\PowerShell\PSReadLine\ConsoleHost_history.txt",
        ),
    ]
}

fn parse_shell_history_line(line: &str) -> String {
    let candidate = if let Some((_, command)) = line.split_once(';') {
        command
    } else {
        line
    };

    sanitize_command(candidate)
}

fn dedupe_and_trim(commands: Vec<String>, limit: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for command in commands {
        if command.is_empty() {
            continue;
        }

        if seen.insert(command.clone()) {
            deduped.push(command);
        }

        if deduped.len() == limit {
            break;
        }
    }

    deduped
}

fn redact_assignment(command: &str) -> String {
    const SENSITIVE_KEYS: [&str; 8] = [
        "GEMINI_API_KEY",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "API_KEY",
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASS",
    ];

    for key in SENSITIVE_KEYS {
        let assignment = format!("{key}=");
        if let Some(index) = command.find(&assignment) {
            let prefix = &command[..index + assignment.len()];
            return format!("{prefix}[REDACTED]");
        }
    }

    command.to_string()
}

fn redact_setx(command: &str) -> String {
    let lower = command.to_ascii_lowercase();
    if !lower.starts_with("setx ") {
        return command.to_string();
    }

    let mut parts = command.split_whitespace();
    let Some(setx) = parts.next() else {
        return command.to_string();
    };
    let Some(variable) = parts.next() else {
        return command.to_string();
    };

    let upper_variable = variable.to_ascii_uppercase();
    let is_sensitive = ["KEY", "TOKEN", "SECRET", "PASSWORD", "PASS"]
        .iter()
        .any(|needle| upper_variable.contains(needle));

    if !is_sensitive {
        return command.to_string();
    }

    format!("{setx} {variable} [REDACTED]")
}
