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
            .unwrap_or_else(|| "commit subject unavailable".to_string());

        println!(
            "{}  {}  {} commands  {} files",
            context.commit,
            context.timestamp,
            context.commands.len(),
            context.files.len()
        );
        println!("  {}", subject);

        if !context.files.is_empty() {
            println!("  Files: {}", context.files.join(", "));
        }
        println!();
    }
}
