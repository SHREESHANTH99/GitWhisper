use serde::Serialize;
use chrono::Utc;
use std::fs;

#[derive(Serialize)]
pub struct CommitContext {
    pub commit: String,
    pub timestamp: String,
    pub commands: Vec<String>,
    pub environment: String,
}

// Save context into .git/commitlens/<commit>.json
pub fn save_context_full(commit: &str, commands: Vec<String>, environment: String) {
    let context = CommitContext {
        commit: commit.to_string(),
        timestamp: Utc::now().to_rfc3339(),
        commands,
        environment,
    };

    fs::create_dir_all(".git/commitlens").unwrap();
    let file_path = format!(".git/commitlens/{}.json", commit);
    let json = serde_json::to_string_pretty(&context).unwrap();
    fs::write(file_path, json).unwrap();
    println!("Saved context for commit {}", commit);
}

// Optional: show all logs
pub fn show_logs() {
    let dir = ".git/commitlens";
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            let file = entry.unwrap().path();
            let data = fs::read_to_string(file).unwrap();
            println!("{}", data);
        }
    }
}