use crate::git;
use std::env;
use std::process::Command;

pub fn collect_environment() -> String {
    let os = env::consts::OS;
    let branch = git::current_branch().unwrap_or_else(|| "unknown".to_string());
    let shell = env::var("SHELL")
        .or_else(|_| env::var("ComSpec"))
        .unwrap_or_else(|_| "unknown".to_string());
    let cwd = env::current_dir()
        .ok()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let lines = vec![
        format!("OS: {os}"),
        format!("Branch: {branch}"),
        format!("Shell: {shell}"),
        format!("Working Directory: {cwd}"),
        format!(
            "Node: {}",
            version_for("node", &["-v"]).unwrap_or_else(|| "not installed".to_string())
        ),
        format!(
            "Python: {}",
            version_for("python", &["--version"]).unwrap_or_else(|| "not installed".to_string())
        ),
        format!(
            "Rust: {}",
            version_for("rustc", &["--version"]).unwrap_or_else(|| "not installed".to_string())
        ),
    ];

    lines.join("\n")
}

fn version_for(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !stdout.is_empty() {
        Some(stdout)
    } else if !stderr.is_empty() {
        Some(stderr)
    } else {
        None
    }
}
