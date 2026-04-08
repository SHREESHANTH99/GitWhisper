pub fn show_timeline(file: &str) {
    let history = match crate::git::file_history(file, 15) {
        Ok(history) => history,
        Err(error) => {
            eprintln!("Could not load timeline for `{}`: {}", file, error);
            return;
        }
    };

    if history.is_empty() {
        println!("No Git history found for {}.", file);
        return;
    }

    println!("{} timeline:\n", file);
    for entry in history {
        println!("{}  {}", entry.short_hash, entry.subject);
    }
}
