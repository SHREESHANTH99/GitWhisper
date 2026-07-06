use super::output;
use crate::spinner::Spinner;
use colored::Colorize;

pub fn show_quality(path: &str) -> crate::error::AppResult<()> {
    let spin = Spinner::new(format!("Analyzing code quality for {}…", path));

    let report = match crate::analysis::quality_analyzer::analyze_target(path) {
        Ok(report) => {
            spin.success(format!(
                "Quality analysis complete — {} file{} checked",
                report.files_analyzed,
                if report.files_analyzed == 1 { "" } else { "s" }
            ));
            report
        }
        Err(error) => {
            let msg = format!("Analysis failed: {error}");
            spin.fail(&msg);
            return Err(crate::error::AppError::message(msg));
        }
    };

    println!();
    output::section_header("🔍", "Code Quality", &report.target);
    println!();

    // ── Stats row ─────────────────────────────────────────────────────────
    println!(
        "  Overall risk: {}  {}  Files analyzed: {}",
        output::risk_colored(report.overall_risk),
        output::risk_bar(report.overall_risk),
        report.files_analyzed.to_string().white().bold()
    );

    // ── Top-level findings ────────────────────────────────────────────────
    if !report.findings.is_empty() {
        println!();
        output::subsection("Findings");
        output::print_findings(&report.findings, report.findings.len());
    }

    // ── Per-file breakdown ────────────────────────────────────────────────
    println!();
    for file_report in &report.file_reports {
        let bar = output::risk_bar(file_report.risk_score);
        println!(
            "  {} {}",
            "▶".bright_black(),
            output::file_path(&file_report.path)
        );
        println!(
            "    Risk: {} {}   LOC: {}  Complexity: {}  Churn: {}  Bug-fix commits: {}  Owners: {}  ({:.0}% top)",
            output::risk_colored(file_report.risk_score),
            bar,
            file_report.approx_loc.to_string().bright_black(),
            file_report.approx_complexity.to_string().bright_black(),
            file_report.recent_churn.to_string().bright_black(),
            file_report.bug_fix_commits.to_string().bright_black(),
            file_report.unique_authors.to_string().bright_black(),
            file_report.top_owner_share * 100.0
        );

        if file_report.duplicate_lines > 0 || file_report.commit_count > 0 {
            println!(
                "    {} {} duplicate lines · {} recent commits",
                "≈".bright_black(),
                file_report.duplicate_lines.to_string().bright_black(),
                file_report.commit_count.to_string().bright_black()
            );
        }

        output::print_findings(&file_report.findings, 3);

        if let Some(suggestion) = file_report.suggestions.first() {
            output::print_suggestion(suggestion);
        }
        println!();
    }

    // ── Recommended next steps ────────────────────────────────────────────
    output::print_next_steps(&report.suggestions);

    Ok(())
}
