use crate::analysis::{codebase_insights, performance_analyzer, quality_analyzer, security_analyzer};
use crate::error::AppResult;

#[derive(Debug, Clone)]
pub struct BugPredictionReport {
    pub target: String,
    pub files_analyzed: usize,
    pub predictions: Vec<BugPrediction>,
}

#[derive(Debug, Clone)]
pub struct BugPrediction {
    pub path: String,
    pub bug_likelihood: u32,
    pub bug_fix_commits: usize,
    pub recent_churn: usize,
    pub complexity: u32,
    pub reasons: Vec<String>,
}

pub fn analyze_target(path: &str, limit: usize) -> AppResult<BugPredictionReport> {
    let (root, normalized) = codebase_insights::normalize_target(path)?;
    let files = codebase_insights::collect_target_files(&root, &normalized)?;

    let mut predictions = files
        .iter()
        .filter_map(|file| {
            let insight = codebase_insights::analyze_file(&root, file).ok()?;
            let quality = quality_analyzer::report_for_insight(file, &insight);
            let security = security_analyzer::report_for_insight(file, &insight);
            let performance = performance_analyzer::report_for_insight(file, &insight);

            let mut score = 0u32;
            let mut reasons = Vec::new();

            if insight.bug_fix_commits >= 3 {
                score += 30;
                reasons.push(format!(
                    "{} recent bug-fix commits touch this file.",
                    insight.bug_fix_commits
                ));
            }
            if insight.recent_churn >= 120 {
                score += 24;
                reasons.push(format!(
                    "Recent churn is high at {} changed lines.",
                    insight.recent_churn
                ));
            }
            if insight.approx_complexity >= 45 {
                score += 18;
                reasons.push(format!(
                    "Complexity is high with a score of {}.",
                    insight.approx_complexity
                ));
            }
            if insight.history.len() <= 3 && insight.recent_churn >= 60 {
                score += 12;
                reasons.push("The file is still young but already changing heavily.".to_string());
            }
            if quality.risk_score >= 50 {
                score += 10;
                reasons.push("Quality signals are already in a high-risk band.".to_string());
            }
            if security.risk_score >= 40 {
                score += 8;
                reasons.push("Security-sensitive heuristics increase defect risk.".to_string());
            }
            if performance.risk_score >= 40 {
                score += 8;
                reasons.push("Performance complexity raises regression risk.".to_string());
            }

            if score == 0 {
                return Some(BugPrediction {
                    path: file.clone(),
                    bug_likelihood: 0,
                    bug_fix_commits: insight.bug_fix_commits,
                    recent_churn: insight.recent_churn,
                    complexity: insight.approx_complexity,
                    reasons: vec!["No strong bug-prediction signal stands out yet.".to_string()],
                });
            }

            Some(BugPrediction {
                path: file.clone(),
                bug_likelihood: score.min(100),
                bug_fix_commits: insight.bug_fix_commits,
                recent_churn: insight.recent_churn,
                complexity: insight.approx_complexity,
                reasons,
            })
        })
        .collect::<Vec<_>>();

    predictions.sort_by(|left, right| {
        right
            .bug_likelihood
            .cmp(&left.bug_likelihood)
            .then_with(|| right.recent_churn.cmp(&left.recent_churn))
            .then_with(|| left.path.cmp(&right.path))
    });

    let files_analyzed = predictions.len();
    if predictions.len() > limit {
        predictions.truncate(limit);
    }

    Ok(BugPredictionReport {
        target: normalized,
        files_analyzed,
        predictions,
    })
}
