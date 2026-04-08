pub fn capture_context() {
    let Some(commit) = crate::git::short_commit_hash() else {
        println!("No commit found yet.");
        return;
    };

    let timestamp = chrono::Utc::now().to_rfc3339();
    let files = crate::git::changed_files_for_commit(&commit).unwrap_or_default();
    let commands = crate::collectors::commands::default_recent_commands();
    let environment = crate::collectors::env::collect_environment();

    let context = crate::storage::context::CommitContext {
        commit: commit.clone(),
        timestamp,
        commands,
        environment,
        files,
    };

    match crate::storage::save::save_context(&context) {
        Ok(path) => {
            println!("Captured context for commit {}", commit);
            println!("Saved metadata to {}", path.display());
        }
        Err(error) => {
            eprintln!("Failed to save commit context: {error}");
        }
    }
}
