use crate::analysis::codebase_insights;
use crate::error::AppResult;
use std::collections::HashSet;

const MAX_FILES_PER_DIRECTORY_REPORT: usize = 12;

#[derive(Debug, Clone)]
pub struct QualityReport {
    pub target: String,
    pub files_analyzed: usize,
    pub overall_risk: u32,
    pub file_reports: Vec<FileQualityReport>,
    pub findings: Vec<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FileQualityReport {
    pub path: String,
    pub approx_loc: usize,
    pub commit_count: usize,
    pub unique_authors: usize,
    pub top_owner_share: f64,
    pub recent_churn: usize,
    pub bug_fix_commits: usize,
    pub approx_complexity: u32,
    pub duplicate_lines: usize,
    pub risk_score: u32,
    pub findings: Vec<String>,
    pub suggestions: Vec<String>,
}

pub fn analyze_target(path: &str) -> AppResult<QualityReport> {
    let (root, normalized) = codebase_insights::normalize_target(path)?;
    let files = codebase_insights::collect_target_files(&root, &normalized)?;
    let is_file = files.len() == 1 && files[0] == normalized;

    let mut file_reports = files
        .iter()
        .filter_map(|file| analyze_file(&root, file).ok())
        .collect::<Vec<_>>();

    file_reports.sort_by(|left, right| {
        right
            .risk_score
            .cmp(&left.risk_score)
            .then_with(|| right.recent_churn.cmp(&left.recent_churn))
            .then_with(|| left.path.cmp(&right.path))
    });

    let files_analyzed = file_reports.len();
    let overall_risk = average_risk(&file_reports);

    if is_file {
        let report = file_reports
            .into_iter()
            .next()
            .expect("single file report exists");
        return Ok(QualityReport {
            target: normalized,
            files_analyzed: 1,
            overall_risk: report.risk_score,
            findings: report.findings.clone(),
            suggestions: report.suggestions.clone(),
            file_reports: vec![report],
        });
    }

    let findings = build_directory_findings(&file_reports);
    let suggestions = build_directory_suggestions(&file_reports);
    if file_reports.len() > MAX_FILES_PER_DIRECTORY_REPORT {
        file_reports.truncate(MAX_FILES_PER_DIRECTORY_REPORT);
    }

    Ok(QualityReport {
        target: normalized,
        files_analyzed,
        overall_risk,
        file_reports,
        findings,
        suggestions,
    })
}

fn analyze_file(root: &std::path::Path, normalized_file: &str) -> AppResult<FileQualityReport> {
    let insight = codebase_insights::analyze_file(root, normalized_file)?;
    Ok(report_for_insight(normalized_file, &insight))
}

pub(crate) fn report_for_insight(
    normalized_file: &str,
    insight: &codebase_insights::FileInsight,
) -> FileQualityReport {
    let mut findings = Vec::new();
    let mut suggestions = Vec::new();
    let mut risk_score = 0u32;

    if insight.approx_complexity >= 45 {
        findings.push(format!(
            "{} has very high control-flow complexity (score {}).",
            normalized_file, insight.approx_complexity
        ));
        suggestions.push(format!(
            "Split `{}` into smaller functions or modules to reduce branching pressure.",
            normalized_file
        ));
        risk_score += 30;
    } else if insight.approx_complexity >= 25 {
        findings.push(format!(
            "{} is showing moderate complexity growth (score {}).",
            normalized_file, insight.approx_complexity
        ));
        suggestions.push(format!(
            "Review large functions in `{}` and extract focused helpers where possible.",
            normalized_file
        ));
        risk_score += 18;
    }

    if insight.duplicate_lines >= 6 {
        findings.push(format!(
            "{} contains repeated logic patterns across {} significant lines.",
            normalized_file, insight.duplicate_lines
        ));
        suggestions.push(format!(
            "Extract repeated logic in `{}` into a shared helper or utility.",
            normalized_file
        ));
        risk_score += 18;
    } else if insight.duplicate_lines >= 3 {
        findings.push(format!(
            "{} has early duplication signals across {} lines.",
            normalized_file, insight.duplicate_lines
        ));
        risk_score += 8;
    }

    if insight.recent_churn >= 160 {
        findings.push(format!(
            "{} is a heavy-change hotspot with {} changed lines across recent commits.",
            normalized_file, insight.recent_churn
        ));
        suggestions.push(format!(
            "Stabilize `{}` with focused cleanup before layering more features on top.",
            normalized_file
        ));
        risk_score += 24;
    } else if insight.recent_churn >= 80 {
        findings.push(format!(
            "{} is seeing elevated churn ({} recent changed lines).",
            normalized_file, insight.recent_churn
        ));
        risk_score += 12;
    }

    if insight.bug_fix_commits >= 3 {
        findings.push(format!(
            "{} appears in {} recent bug-fix commits.",
            normalized_file, insight.bug_fix_commits
        ));
        suggestions.push(format!(
            "Add stronger regression coverage around `{}` because it has repeated bug-fix history.",
            normalized_file
        ));
        risk_score += 16;
    }

    if insight.top_owner_share >= 0.80 && insight.history.len() >= 5 {
        findings.push(format!(
            "{} has a knowledge-silo risk with one owner covering {:.0}% of commits.",
            normalized_file,
            insight.top_owner_share * 100.0
        ));
        suggestions.push(format!(
            "Spread knowledge on `{}` through pairing or broader review ownership.",
            normalized_file
        ));
        risk_score += 12;
    } else if insight.top_owner_share >= 0.60 && insight.history.len() >= 5 {
        findings.push(format!(
            "{} has concentrated ownership at {:.0}% for the top contributor.",
            normalized_file,
            insight.top_owner_share * 100.0
        ));
        risk_score += 6;
    }

    if insight.approx_loc >= 400 {
        findings.push(format!(
            "{} is large ({} non-empty lines), which raises maintenance cost.",
            normalized_file, insight.approx_loc
        ));
        suggestions.push(format!(
            "Consider breaking `{}` into smaller focused units if responsibilities are mixed.",
            normalized_file
        ));
        risk_score += 10;
    }

    if findings.is_empty() {
        findings.push(format!(
            "{} looks relatively stable with no strong quality red flags from current heuristics.",
            normalized_file
        ));
    }

    if suggestions.is_empty() {
        suggestions.push(format!(
            "Keep `{}` under observation as more commit history accumulates.",
            normalized_file
        ));
    }

    FileQualityReport {
        path: normalized_file.to_string(),
        approx_loc: insight.approx_loc,
        commit_count: insight.history.len(),
        unique_authors: insight.unique_authors,
        top_owner_share: insight.top_owner_share,
        recent_churn: insight.recent_churn,
        bug_fix_commits: insight.bug_fix_commits,
        approx_complexity: insight.approx_complexity,
        duplicate_lines: insight.duplicate_lines,
        risk_score: risk_score.min(100),
        findings,
        suggestions,
    }
}

fn average_risk(file_reports: &[FileQualityReport]) -> u32 {
    if file_reports.is_empty() {
        return 0;
    }

    let total: u32 = file_reports.iter().map(|report| report.risk_score).sum();
    total / file_reports.len() as u32
}

fn build_directory_findings(file_reports: &[FileQualityReport]) -> Vec<String> {
    let mut findings = Vec::new();

    let high_risk = file_reports
        .iter()
        .filter(|report| report.risk_score >= 50)
        .count();
    if high_risk > 0 {
        findings.push(format!(
            "{} files in this directory are already in a high-risk band.",
            high_risk
        ));
    }

    let hotspot_files = file_reports
        .iter()
        .filter(|report| report.recent_churn >= 80)
        .map(|report| report.path.clone())
        .take(3)
        .collect::<Vec<_>>();
    if !hotspot_files.is_empty() {
        findings.push(format!(
            "Hotspots are concentrated in {}.",
            hotspot_files.join(", ")
        ));
    }

    let silo_files = file_reports
        .iter()
        .filter(|report| report.top_owner_share >= 0.80)
        .map(|report| report.path.clone())
        .take(3)
        .collect::<Vec<_>>();
    if !silo_files.is_empty() {
        findings.push(format!(
            "Knowledge-silo risk appears in {}.",
            silo_files.join(", ")
        ));
    }

    if findings.is_empty() {
        findings.push(
            "No broad directory-level quality risk stands out from current heuristics.".to_string(),
        );
    }

    findings
}

fn build_directory_suggestions(file_reports: &[FileQualityReport]) -> Vec<String> {
    let mut suggestions = Vec::new();
    let top_risky = file_reports.iter().take(3).collect::<Vec<_>>();

    if !top_risky.is_empty() {
        suggestions.push(format!(
            "Start refactoring with {} because they currently carry the highest combined risk.",
            top_risky
                .iter()
                .map(|report| report.path.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let repeated_bug_fix = file_reports
        .iter()
        .filter(|report| report.bug_fix_commits >= 3)
        .map(|report| report.path.clone())
        .collect::<HashSet<_>>();
    if !repeated_bug_fix.is_empty() {
        suggestions.push(format!(
            "Add regression coverage for {} because these files show repeated bug-fix churn.",
            repeated_bug_fix.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    if suggestions.is_empty() {
        suggestions.push(
            "Keep collecting commit context so future quality reports can give sharper recommendations."
                .to_string(),
        );
    }

    suggestions
}
