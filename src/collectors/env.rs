use std::env;

pub fn collect_env() -> (Option<String>, String) {

    let node = std::process::Command::new("node")
        .arg("-v")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string());

    let os = env::consts::OS.to_string();

    (node, os)
}