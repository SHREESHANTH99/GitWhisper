use crate::analysis::codebase_insights;
use crate::error::AppResult;

const MAX_FILES_PER_REPORT: usize = 12;

#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub target: String,
    pub files_analyzed: usize,
    pub overall_risk: u32,
    pub file_reports: Vec<FilePerformanceReport>,
    pub findings: Vec<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FilePerformanceReport {
    pub path: String,
    pub risk_score: u32,
    pub approx_complexity: u32,
    pub nested_loop_signals: usize,
    pub allocation_signals: usize,
    pub io_signals: usize,
    pub recent_churn: usize,
    pub findings: Vec<String>,
    pub suggestions: Vec<String>,
}

pub fn analyze_target(path: &str) -> AppResult<PerformanceReport> {
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
        return Ok(PerformanceReport {
            target: normalized,
            files_analyzed: 1,
            overall_risk: report.risk_score,
            findings: report.findings.clone(),
            suggestions: report.suggestions.clone(),
            file_reports: vec![report],
        });
    }

    let findings = build_findings(&file_reports);
    let suggestions = build_suggestions(&file_reports);
    if file_reports.len() > MAX_FILES_PER_REPORT {
        file_reports.truncate(MAX_FILES_PER_REPORT);
    }

    Ok(PerformanceReport {
        target: normalized,
        files_analyzed,
        overall_risk,
        file_reports,
        findings,
        suggestions,
    })
}

fn analyze_file(root: &std::path::Path, normalized_file: &str) -> AppResult<FilePerformanceReport> {
    let insight = codebase_insights::analyze_file(root, normalized_file)?;
    Ok(report_for_insight(normalized_file, &insight))
}

pub(crate) fn report_for_insight(
    normalized_file: &str,
    insight: &codebase_insights::FileInsight,
) -> FilePerformanceReport {
    let content = insight.content.to_ascii_lowercase();

    let nested_loop_signals = count_occurrences(
        &content,
        &["for ", "while ", ".iter().map(", ".iter().filter("],
    );
    let allocation_signals = count_occurrences(
        &content,
        &[
            ".clone()",
            "to_string()",
            "collect::<vec<_>>()",
            "vec::new()",
            "string::new()",
        ],
    );
    let io_signals = count_occurrences(
        &content,
        &[
            "fs::read",
            "fs::write",
            "read_to_string",
            "write_all",
            "println!",
            "eprintln!",
        ],
    );

    let mut findings = Vec::new();
    let mut suggestions = Vec::new();
    let mut risk_score = 0u32;

    if insight.approx_complexity >= 45 {
        findings.push(format!(
            "{} has high branching complexity that can amplify runtime cost.",
            normalized_file
        ));
        suggestions.push(format!(
            "Simplify hot paths in `{}` before adding more logic.",
            normalized_file
        ));
        risk_score += 20;
    }

    if nested_loop_signals >= 6 {
        findings.push(format!(
            "{} shows repeated nested iteration patterns that may hide O(n^2) behavior.",
            normalized_file
        ));
        suggestions.push(format!(
            "Review loops in `{}` for precomputed maps, indexes, or caching.",
            normalized_file
        ));
        risk_score += 24;
    } else if nested_loop_signals >= 3 {
        findings.push(format!(
            "{} has moderate iteration pressure from repeated loops or chained iterators.",
            normalized_file
        ));
        risk_score += 10;
    }

    if allocation_signals >= 10 {
        findings.push(format!(
            "{} performs frequent cloning or allocation-style operations.",
            normalized_file
        ));
        suggestions.push(format!(
            "Reduce cloning and intermediate allocations in `{}` where ownership allows.",
            normalized_file
        ));
        risk_score += 18;
    } else if allocation_signals >= 5 {
        findings.push(format!(
            "{} has some allocation churn from repeated string or vector construction.",
            normalized_file
        ));
        risk_score += 8;
    }

    if io_signals >= 6 && insight.approx_complexity >= 25 {
        findings.push(format!(
            "{} mixes I/O-heavy operations with complex control flow.",
            normalized_file
        ));
        suggestions.push(format!(
            "Separate I/O from decision logic in `{}` to make performance tuning easier.",
            normalized_file
        ));
        risk_score += 16;
    }

    if insight.recent_churn >= 120 {
        findings.push(format!(
            "{} is changing quickly, which makes performance regressions easier to introduce.",
            normalized_file
        ));
        risk_score += 12;
    }

    if findings.is_empty() {
        findings.push(format!(
            "{} has no strong performance warning signs from current heuristics.",
            normalized_file
        ));
        suggestions.push(format!(
            "Keep `{}` under observation as performance-sensitive history accumulates.",
            normalized_file
        ));
    }

    FilePerformanceReport {
        path: normalized_file.to_string(),
        risk_score: risk_score.min(100),
        approx_complexity: insight.approx_complexity,
        nested_loop_signals,
        allocation_signals,
        io_signals,
        recent_churn: insight.recent_churn,
        findings,
        suggestions,
    }
}

fn count_occurrences(content: &str, patterns: &[&str]) -> usize {
    patterns
        .iter()
        .map(|pattern| content.matches(pattern).count())
        .sum()
}

fn average_risk(file_reports: &[FilePerformanceReport]) -> u32 {
    if file_reports.is_empty() {
        return 0;
    }

    let total: u32 = file_reports.iter().map(|report| report.risk_score).sum();
    total / file_reports.len() as u32
}

fn build_findings(file_reports: &[FilePerformanceReport]) -> Vec<String> {
    let hotspots = file_reports
        .iter()
        .filter(|report| report.risk_score >= 35)
        .map(|report| report.path.clone())
        .take(4)
        .collect::<Vec<_>>();
    if hotspots.is_empty() {
        return vec![
            "No broad directory-level performance hotspot stands out from current heuristics."
                .to_string(),
        ];
    }

    vec![format!(
        "Potential performance hotspots are concentrated in {}.",
        hotspots.join(", ")
    )]
}

fn build_suggestions(file_reports: &[FilePerformanceReport]) -> Vec<String> {
    let top = file_reports.iter().take(3).collect::<Vec<_>>();
    if top.is_empty() {
        return vec![
            "Keep collecting commit context to improve future performance reports.".to_string(),
        ];
    }

    vec![format!(
        "Start profiling or reviewing {} because they currently carry the highest heuristic performance risk.",
        top.iter()
            .map(|report| report.path.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )]
}
