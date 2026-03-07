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

pub fn save_context_full(commit: &str, commands: Vec<String>, environment: String) {
    let context = CommitContext {
        commit: commit.to_string(),
        timestamp: Utc::now().to_rfc3339(),
        commands,
        environment,
    };

    fs::create_dir_all(".git/git-insight").unwrap();
    let file = format!(".git/git-insight/{}.json", commit);
    let json = serde_json::to_string_pretty(&context).unwrap();
    fs::write(file, json).unwrap();
    println!("Saved full context for commit {}", commit);
}
pub fn show_logs() {
    let dir = ".git/git-insight";
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(content) = fs::read_to_string(&path) {
                println!("{}", content);
            }
        }
    } else {
        println!("No git-insight context found yet.");
    }
}