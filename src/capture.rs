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
    let ide = crate::collectors::ide::collect_ide_context(&files);
    let review = crate::collectors::review_context::collect_review_context(&commands);
    let behavior = crate::analysis::behavior_patterns::analyze_author_patterns(&commit, &files)
        .unwrap_or_default();
    let analysis = if config.capture.include_analysis {
        let commit_message =
            crate::git::commit_message(&commit).unwrap_or_else(|_| crate::git::CommitMessage {
                subject: String::new(),
                body: String::new(),
            });
        let diff = crate::analysis::diff_parser::summarize_commit(&commit).unwrap_or_default();
        let diff = if diff.files_changed == 0 && !files.is_empty() {
            let mut adjusted = diff;
            adjusted.files_changed = files.len();
            adjusted
        } else {
            diff
        };
        let impact =
            crate::analysis::impact_analysis::analyze_impact(&files, &diff).unwrap_or_default();

        crate::analysis::CommitAnalysis {
            intent: crate::analysis::intent_detection::classify_commit_intent(
                &commit_message.subject,
                &commit_message.body,
                &diff,
            ),
            diff,
            impact,
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
        ide,
        review,
        behavior,
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
