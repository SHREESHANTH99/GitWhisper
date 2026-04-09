pub fn show_logs() {
    let contexts = match crate::storage::load::load_all_contexts() {
        Ok(contexts) => contexts,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    if contexts.is_empty() {
        println!("No captured commit context found yet.");
        return;
    }

    println!("Captured commit contexts:\n");
    for context in contexts {
        let subject = crate::git::commit_subject(&context.commit)
            .unwrap_or_else(|_| "commit subject unavailable".to_string());

        println!(
            "{}  {}  {} commands  {} files",
            context.commit,
            context.timestamp,
            context.commands.len(),
            context.files.len()
        );
        println!("  {}", subject);
        if !context.analysis.is_empty() {
            println!("  Intent: {}", context.analysis.intent.summary());
            if let Some(signals) = context.analysis.intent.signals_summary(3) {
                println!("  Signals: {}", signals);
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
        }

        if let Some(summary) = context.ide.summary() {
            println!("  IDE: {}", summary);
        }
        if let Some(summary) = context.review.summary() {
            println!("  Review: {}", summary);
        }
        if let Some(summary) = context.behavior.summary() {
            println!("  Behavior: {}", summary);
        }

        if !context.files.is_empty() {
            println!("  Files: {}", context.files.join(", "));
        }
        println!();
    }
}
