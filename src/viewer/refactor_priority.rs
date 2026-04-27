pub fn show_refactor_priority(path: Option<&str>, limit: usize) {
    let target = path.unwrap_or(".");
    let report = match crate::analysis::refactor_prioritizer::analyze_target(target, limit.max(1)) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    println!(
        "Refactor Priority Report for {}\n",
        report.target
    );
    println!(
        "Files ranked: {} | Showing top {}\n",
        report.files_analyzed,
        report.priorities.len()
    );

    for (index, item) in report.priorities.iter().enumerate() {
        println!(
            "{:>2}. {}\n    Total: {} | Quality: {} | Security: {} | Performance: {}",
            index + 1,
            item.path,
            item.total_score,
            item.quality_score,
            item.security_score,
            item.performance_score
        );

        for reason in item.reasons.iter().take(2) {
            println!("    Reason: {}", reason);
        }
        println!("    Next: {}\n", item.next_step);
    }
}
