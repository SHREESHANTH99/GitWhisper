pub fn show_security(path: &str) {
    let report = match crate::analysis::security_analyzer::analyze_target(path) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    println!("Security Report for {}\n", report.target);
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
            "{}\n  Risk: {}/100 | Secrets: {} | Injection: {} | Shell exec: {} | Crypto: {} | Auth markers: {}",
            file_report.path,
            file_report.risk_score,
            file_report.secret_hits,
            file_report.injection_hits,
            file_report.shell_execution_hits,
            file_report.crypto_hits,
            file_report.auth_hits
        );
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
