use crate::analysis::codebase_insights;
use crate::error::AppResult;

#[derive(Debug, Clone)]
pub struct KnowledgeRiskReport {
    pub target: String,
    pub files_analyzed: usize,
    pub entries: Vec<KnowledgeRiskEntry>,
}

#[derive(Debug, Clone)]
pub struct KnowledgeRiskEntry {
    pub path: String,
    pub risk_score: u32,
    pub unique_authors: usize,
    pub top_owner_share: f64,
    pub history_depth: usize,
    pub last_commit: String,
    pub reasons: Vec<String>,
    pub mitigation: String,
}

pub fn analyze_target(path: &str, limit: usize) -> AppResult<KnowledgeRiskReport> {
    let (root, normalized) = codebase_insights::normalize_target(path)?;
    let files = codebase_insights::collect_target_files(&root, &normalized)?;

    let mut entries = files
        .iter()
        .filter_map(|file| {
            let insight = codebase_insights::analyze_file(&root, file).ok()?;
            let mut risk_score = 0u32;
            let mut reasons = Vec::new();

            if insight.top_owner_share >= 0.90 && insight.history.len() >= 4 {
                risk_score += 35;
                reasons.push(format!(
                    "One contributor owns {:.0}% of observed commits.",
                    insight.top_owner_share * 100.0
                ));
            } else if insight.top_owner_share >= 0.70 && insight.history.len() >= 4 {
                risk_score += 18;
                reasons.push(format!(
                    "Ownership is concentrated at {:.0}% for the top contributor.",
                    insight.top_owner_share * 100.0
                ));
            }

            if insight.unique_authors <= 1 && insight.history.len() >= 4 {
                risk_score += 24;
                reasons.push(
                    "Only one contributor appears in the observed ownership history.".to_string(),
                );
            } else if insight.unique_authors == 2 && insight.history.len() >= 6 {
                risk_score += 10;
                reasons.push("Knowledge is mostly held by only two contributors.".to_string());
            }

            if insight.approx_loc >= 300 && insight.unique_authors <= 2 {
                risk_score += 12;
                reasons
                    .push("The file is large while knowledge spread remains narrow.".to_string());
            }

            let last_commit = insight
                .history
                .first()
                .map(|entry| entry.commit.timestamp.clone())
                .unwrap_or_else(|| "unknown".to_string());

            let mitigation = if risk_score >= 40 {
                format!(
                    "Prioritize cross-training and broaden review ownership for `{}`.",
                    file
                )
            } else if risk_score >= 20 {
                format!(
                    "Add at least one more regular reviewer for `{}` to reduce concentration.",
                    file
                )
            } else {
                format!(
                    "Knowledge distribution for `{}` looks manageable right now.",
                    file
                )
            };

            if reasons.is_empty() {
                reasons.push("No strong knowledge-silo signal stands out yet.".to_string());
            }

            Some(KnowledgeRiskEntry {
                path: file.clone(),
                risk_score: risk_score.min(100),
                unique_authors: insight.unique_authors,
                top_owner_share: insight.top_owner_share,
                history_depth: insight.history.len(),
                last_commit,
                reasons,
                mitigation,
            })
        })
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        right
            .risk_score
            .cmp(&left.risk_score)
            .then_with(|| left.unique_authors.cmp(&right.unique_authors))
            .then_with(|| left.path.cmp(&right.path))
    });

    let files_analyzed = entries.len();
    if entries.len() > limit {
        entries.truncate(limit);
    }

    Ok(KnowledgeRiskReport {
        target: normalized,
        files_analyzed,
        entries,
    })
}
