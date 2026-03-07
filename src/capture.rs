use crate::git;
use crate::storage;
use std::fs;
use std::process::Command;
use dirs::home_dir;

// Capture recent terminal commands on Windows + Linux
pub fn recent_commands() -> Vec<String> {
    let mut commands = Vec::new();

    if let Some(home) = home_dir() {
        // Git Bash / Linux Bash
        let bash_history = home.join(".bash_history");
        if let Ok(content) = fs::read_to_string(bash_history) {
            commands.extend(content.lines().rev().take(20).map(|l| l.to_string()));
        }

        // Windows PowerShell
        let ps_history = home.join(r"AppData\Roaming\Microsoft\Windows\PowerShell\PSReadLine\ConsoleHost_history.txt");
        if let Ok(content) = fs::read_to_string(ps_history) {
            commands.extend(content.lines().rev().take(20).map(|l| l.to_string()));
        }
    }

    commands
}

// Capture environment info
pub fn environment_info() -> String {
    let os = std::env::consts::OS;
    let branch = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into());

    let node_version = Command::new("node")
        .arg("-v")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "not installed".into());

    let python_version = Command::new("python")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "not installed".into());

    format!(
        "OS: {}\nBranch: {}\nNode: {}\nPython: {}",
        os, branch, node_version, python_version
    )
}

// Capture everything and save
pub fn capture_context() {
    if let Some(commit) = git::short_commit_hash() {
        println!("Capturing context for commit {}", commit);
        let commands = recent_commands();
        let env = environment_info();
        storage::save_context_full(&commit, commands, env);
    } else {
        println!("No commit found yet.");
    }
}