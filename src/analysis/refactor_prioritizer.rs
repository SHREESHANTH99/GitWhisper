use crate::analysis::{codebase_insights, performance_analyzer, quality_analyzer, security_analyzer};
use crate::error::AppResult;

#[derive(Debug, Clone)]
pub struct RefactorPriorityReport {
    pub target: String,
    pub files_analyzed: usize,
    pub priorities: Vec<RefactorPriorityItem>,
}

#[derive(Debug, Clone)]
pub struct RefactorPriorityItem {
    pub path: String,
    pub total_score: u32,
    pub quality_score: u32,
    pub security_score: u32,
    pub performance_score: u32,
    pub reasons: Vec<String>,
    pub next_step: String,
}

pub fn analyze_target(path: &str, limit: usize) -> AppResult<RefactorPriorityReport> {
    let (root, normalized) = codebase_insights::normalize_target(path)?;
    let files = codebase_insights::collect_target_files(&root, &normalized)?;

    let mut priorities = files
        .iter()
        .filter_map(|file| {
            let insight = codebase_insights::analyze_file(&root, file).ok()?;
            let quality = quality_analyzer::report_for_insight(file, &insight);
            let security = security_analyzer::report_for_insight(file, &insight);
            let performance = performance_analyzer::report_for_insight(file, &insight);

            let mut reasons = Vec::new();
            if let Some(reason) = quality.findings.first() {
                reasons.push(reason.clone());
            }
            if security.risk_score > 0 {
                if let Some(reason) = security.findings.first() {
                    reasons.push(reason.clone());
                }
            }
            if performance.risk_score > 0 {
                if let Some(reason) = performance.findings.first() {
                    reasons.push(reason.clone());
                }
            }

            let next_step = security
                .suggestions
                .first()
                .filter(|_| security.risk_score > 0)
                .or_else(|| quality.suggestions.first())
                .or_else(|| performance.suggestions.first())
                .cloned()
                .unwrap_or_else(|| "Review the file and simplify its highest-risk behavior first.".to_string());

            Some(RefactorPriorityItem {
                path: file.clone(),
                total_score: quality.risk_score + security.risk_score + performance.risk_score,
                quality_score: quality.risk_score,
                security_score: security.risk_score,
                performance_score: performance.risk_score,
                reasons,
                next_step,
            })
        })
        .collect::<Vec<_>>();

    priorities.sort_by(|left, right| {
        right
            .total_score
            .cmp(&left.total_score)
            .then_with(|| right.security_score.cmp(&left.security_score))
            .then_with(|| right.quality_score.cmp(&left.quality_score))
            .then_with(|| right.performance_score.cmp(&left.performance_score))
            .then_with(|| left.path.cmp(&right.path))
    });

    let files_analyzed = priorities.len();
    if priorities.len() > limit {
        priorities.truncate(limit);
    }

    Ok(RefactorPriorityReport {
        target: normalized,
        files_analyzed,
        priorities,
    })
}
