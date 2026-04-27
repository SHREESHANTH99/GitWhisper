pub fn show_bug_predictions(path: Option<&str>, limit: usize) {
    let target = path.unwrap_or(".");
    let report = match crate::analysis::bug_predictor::analyze_target(target, limit.max(1)) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    println!("Bug Prediction Report for {}\n", report.target);
    println!(
        "Files analyzed: {} | Showing top {}\n",
        report.files_analyzed,
        report.predictions.len()
    );

    for (index, item) in report.predictions.iter().enumerate() {
        println!(
            "{:>2}. {}\n    Likelihood: {}/100 | Bug-fix commits: {} | Churn: {} | Complexity: {}",
            index + 1,
            item.path,
            item.bug_likelihood,
            item.bug_fix_commits,
            item.recent_churn,
            item.complexity
        );
        for reason in item.reasons.iter().take(2) {
            println!("    Reason: {}", reason);
        }
        println!();
    }
}
