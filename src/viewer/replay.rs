use std::fs;
use serde::Deserialize;

#[derive(Deserialize)]
struct CommitContext {
    commit: String,
    timestamp: String,
    commands: Vec<String>,
    environment: String,
}

pub fn replay_commit(commit: &str) {
    let path = format!(".git/git-insight/{}.json", commit);
    let data = fs::read_to_string(&path).expect("Commit context not found");
    let context: CommitContext = serde_json::from_str(&data).unwrap();

    println!("=== Replay for commit {} ===", context.commit);
    println!("Timestamp: {}", context.timestamp);
    println!("Environment:\n{}", context.environment);
    println!("Commands run:");
    for cmd in context.commands.iter().rev() {  // most recent last
        println!("> {}", cmd);
    }
}