pub fn replay_commit(commit: Option<&str>) {
    let context = match commit {
        Some(commit_prefix) => crate::storage::load::load_context(commit_prefix),
        None => crate::storage::load::latest_context(),
    };

    let context = match context {
        Ok(context) => context,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    println!("Replay for commit {}", context.commit);
    if let Ok(subject) = crate::git::commit_subject(&context.commit) {
        println!("Subject: {}", subject);
    }
    println!("Timestamp: {}", context.timestamp);

    if context.files.is_empty() {
        println!("Files: none recorded");
    } else {
        println!("Files:");
        for file in &context.files {
            println!("  - {}", file);
        }
    }

    if context.environment.is_empty() {
        println!("Environment: not captured");
    } else {
        println!("Environment:\n{}", context.environment.to_display_string());
    }

    if context.analysis.is_empty() {
        println!("Analysis: not captured");
    } else {
        println!("Analysis:");
        println!("  Intent: {}", context.analysis.intent.summary());
        if let Some(summary) = context.analysis.diff.summary() {
            println!("  Diff: {}", summary);
        }
        if let Some(summary) = context.analysis.diff.semantic_summary() {
            println!("  Semantic: {}", summary);
        }
        if let Some(files) = context.analysis.diff.top_files_summary(5) {
            println!("  Top files: {files}");
        }
        if let Some(symbols) = context.analysis.diff.changed_symbols_summary(5) {
            println!("  Symbols: {symbols}");
        }
        if let Some(imports) = context.analysis.diff.import_summary(3) {
            println!("  Imports: {imports}");
        }
    }

    if context.commands.is_empty() {
        println!("Commands: none captured");
    } else {
        println!("Commands:");
        for command in context.commands.iter().rev() {
            println!("  > {}", command);
        }
    }
}
