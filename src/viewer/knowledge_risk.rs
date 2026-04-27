pub fn show_knowledge_risk(path: Option<&str>, limit: usize) {
    let target = path.unwrap_or(".");
    let report = match crate::analysis::knowledge_risk::analyze_target(target, limit.max(1)) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    println!("Knowledge Risk Report for {}\n", report.target);
    println!(
        "Files analyzed: {} | Showing top {}\n",
        report.files_analyzed,
        report.entries.len()
    );

    for (index, entry) in report.entries.iter().enumerate() {
        println!(
            "{:>2}. {}\n    Risk: {}/100 | Authors: {} | Top owner: {:.0}% | History: {} commits | Last commit: {}",
            index + 1,
            entry.path,
            entry.risk_score,
            entry.unique_authors,
            entry.top_owner_share * 100.0,
            entry.history_depth,
            entry.last_commit
        );
        for reason in entry.reasons.iter().take(2) {
            println!("    Reason: {}", reason);
        }
        println!("    Mitigation: {}\n", entry.mitigation);
    }
}
