pub fn capture_context() {
    let config = match crate::config::AppConfig::load() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Failed to load .gitwhisper.toml: {error}");
            return;
        }
    };

    let commit = match crate::git::short_commit_hash() {
        Ok(commit) => commit,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    let timestamp = chrono::Utc::now().to_rfc3339();
    let files = crate::git::changed_files_for_commit(&commit).unwrap_or_default();
    let commands = crate::collectors::commands::recent_commands(config.capture.command_limit);
    let environment = if config.capture.include_environment {
        crate::collectors::env::collect_environment()
    } else {
        crate::storage::context::EnvironmentContext::default()
    };
    let analysis = if config.capture.include_analysis {
        let message = crate::git::commit_message(&commit)
            .map(|message| {
                if message.body.trim().is_empty() {
                    message.subject
                } else {
                    format!("{} {}", message.subject, message.body)
                }
            })
            .unwrap_or_default();
        let diff = crate::analysis::diff_parser::summarize_commit(&commit).unwrap_or_default();
        let files_changed = diff.files_changed.max(files.len());

        crate::analysis::CommitAnalysis {
            intent: crate::analysis::intent_detection::classify_commit_message(
                &message,
                files_changed,
            ),
            diff,
        }
    } else {
        crate::analysis::CommitAnalysis::default()
    };

    let context = crate::storage::context::CommitContext {
        schema_version: 2,
        commit: commit.clone(),
        timestamp,
        commands,
        environment,
        files,
        analysis,
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
