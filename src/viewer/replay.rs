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

    if let Some(summary) = context.ide.summary() {
        println!("IDE: {}", summary);
        if !context.ide.extensions.is_empty() {
            println!("  Extensions: {}", context.ide.extensions.join(", "));
        }
        if !context.ide.active_files.is_empty() {
            println!("  Active files: {}", context.ide.active_files.join(", "));
        }
    } else {
        println!("IDE: not captured");
    }

    if let Some(summary) = context.review.summary() {
        println!("Review: {}", summary);
        if !context.review.reviewers.is_empty() {
            println!("  Reviewers: {}", context.review.reviewers.join(", "));
        }
        if !context.review.labels.is_empty() {
            println!("  Labels: {}", context.review.labels.join(", "));
        }
    } else {
        println!("Review: not captured");
    }

    if let Some(summary) = context.behavior.summary() {
        println!("Behavior: {}", summary);
        if !context.behavior.typical_work_hours.is_empty() {
            println!("  Work hours: {}", context.behavior.typical_work_hours);
        }
        if let Some(expertise) = context.behavior.expertise_summary(5) {
            println!("  Expertise: {}", expertise);
        }
    } else {
        println!("Behavior: not captured");
    }

    if context.analysis.is_empty() {
        println!("Analysis: not captured");
    } else {
        println!("Analysis:");
        println!("  Intent: {}", context.analysis.intent.summary());
        if let Some(signals) = context.analysis.intent.signals_summary(5) {
            println!("  Signals: {signals}");
        }
        if let Some(summary) = context.analysis.diff.summary() {
            println!("  Diff: {}", summary);
        }
        if let Some(summary) = context.analysis.diff.semantic_summary() {
            println!("  Semantic: {}", summary);
        }
        if let Some(summary) = context.analysis.impact.summary() {
            println!("  Impact: {}", summary);
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
        if let Some(dependents) = context.analysis.impact.top_direct_summary(5) {
            println!("  Direct impact: {dependents}");
        }
        if let Some(transitive) = context.analysis.impact.top_transitive_summary(5) {
            println!("  Transitive impact: {transitive}");
        }
        if let Some(cycles) = context.analysis.impact.circular_summary(2) {
            println!("  Cycles: {cycles}");
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
