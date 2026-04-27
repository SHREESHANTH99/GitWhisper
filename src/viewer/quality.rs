pub fn show_quality(path: &str) {
    let report = match crate::analysis::quality_analyzer::analyze_target(path) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    println!(
        "Code Quality Report for {}\n",
        report.target
    );
    println!(
        "Overall risk: {}/100 | Files analyzed: {}\n",
        report.overall_risk, report.files_analyzed
    );

    if !report.findings.is_empty() {
        println!("Findings:");
        for finding in &report.findings {
            println!("- {}", finding);
        }
        println!();
    }

    for file_report in &report.file_reports {
        println!(
            "{}\n  Risk: {}/100 | LOC: {} | Complexity: {} | Churn: {} | Bug-fix commits: {} | Owners: {} ({:.0}% top share)",
            file_report.path,
            file_report.risk_score,
            file_report.approx_loc,
            file_report.approx_complexity,
            file_report.recent_churn,
            file_report.bug_fix_commits,
            file_report.unique_authors,
            file_report.top_owner_share * 100.0
        );

        if file_report.duplicate_lines > 0 || file_report.commit_count > 0 {
            println!(
                "  Signals: {} duplicate lines | {} recent commits",
                file_report.duplicate_lines,
                file_report.commit_count
            );
        }

        for finding in file_report.findings.iter().take(3) {
            println!("  Finding: {}", finding);
        }

        if let Some(suggestion) = file_report.suggestions.first() {
            println!("  Suggestion: {}", suggestion);
        }

        println!();
    }

    if !report.suggestions.is_empty() {
        println!("Recommended next steps:");
        for suggestion in &report.suggestions {
            println!("- {}", suggestion);
        }
    }
}
