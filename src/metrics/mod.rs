pub mod exporter;

use crate::analysis::ChangeCategory;
use crate::storage::context::CommitContext;
use chrono::Datelike;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Serialize)]
pub struct AnalyticsSnapshot {
    pub generated_at: String,
    pub overview: OverviewMetrics,
    pub people: Vec<PersonMetrics>,
    pub files: Vec<FileMetrics>,
    pub weekly_activity: Vec<WeeklyActivity>,
    pub risks: Vec<RiskMetric>,
    pub recent_commits: Vec<RecentCommitMetric>,
    /// Commit-count breakdown by detected change category.
    pub intent_breakdown: IntentBreakdown,
}

/// How many commits in this snapshot belong to each change category.
/// The categories map 1-to-1 to [`crate::analysis::ChangeCategory`] variants;
/// `security` is absent from that enum so counts are accumulated under `unknown`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct IntentBreakdown {
    pub feature: usize,
    pub fix: usize,
    pub refactor: usize,
    pub security: usize,
    pub performance: usize,
    pub docs: usize,
    pub unknown: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct OverviewMetrics {
    pub total_commits: usize,
    pub unique_authors: usize,
    pub files_touched: usize,
    pub commits_last_7d: usize,
    pub commits_last_30d: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersonMetrics {
    pub author: String,
    pub commits: usize,
    pub files_touched: usize,
    pub top_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileMetrics {
    pub path: String,
    pub commits: usize,
    pub top_author: String,
    pub top_author_share: f64,
    /// Highest `impact_score` seen across all captured contexts that touched this
    /// file.  `None` when no context with analysis data is available.
    pub risk_score: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeeklyActivity {
    pub week: String,
    pub commits: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RiskMetric {
    pub kind: String,
    pub subject: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentCommitMetric {
    pub commit: String,
    pub timestamp: String,
    pub author: String,
    pub subject: String,
    pub files_changed: usize,
    pub files: Vec<String>,
}

pub fn collect_snapshot() -> crate::error::AppResult<AnalyticsSnapshot> {
    let contexts = crate::storage::load::load_all_contexts()?;
    let git_activity = crate::git::recent_commit_activity(500).unwrap_or_default();
    Ok(collect_snapshot_from_sources(&contexts, &git_activity))
}

pub fn build_digest(period: &str) -> crate::error::AppResult<String> {
    let contexts = crate::storage::load::load_all_contexts()?;
    let now = chrono::Utc::now();
    let cutoff_days = match period {
        "daily" => 1,
        _ => 7,
    };
    let cutoff = now - chrono::Duration::days(cutoff_days);

    let filtered = contexts
        .iter()
        .filter(|context| {
            chrono::DateTime::parse_from_rfc3339(&context.timestamp)
                .map(|value| value.with_timezone(&chrono::Utc) >= cutoff)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();

    let snapshot = collect_snapshot_from_contexts(&filtered);
    let title = if period == "daily" { "Daily" } else { "Weekly" };

    let mut text = format!(
        "{} Gitwhisper Digest\nGenerated: {}\n\n",
        title, snapshot.generated_at
    );
    text.push_str(&format!(
        "Overview: {} commits by {} contributor(s) touching {} file(s).\n\n",
        snapshot.overview.total_commits,
        snapshot.overview.unique_authors,
        snapshot.overview.files_touched
    ));

    if !snapshot.people.is_empty() {
        text.push_str("Top contributors:\n");
        for person in snapshot.people.iter().take(5) {
            text.push_str(&format!(
                "- {}: {} commits across {} files\n",
                person.author, person.commits, person.files_touched
            ));
        }
        text.push('\n');
    }

    if !snapshot.files.is_empty() {
        text.push_str("Hot files:\n");
        for file in snapshot.files.iter().take(5) {
            text.push_str(&format!(
                "- {}: {} commits, top owner {} ({:.0}%)\n",
                file.path,
                file.commits,
                file.top_author,
                file.top_author_share * 100.0
            ));
        }
        text.push('\n');
    }

    if !snapshot.risks.is_empty() {
        text.push_str("Risks:\n");
        for risk in snapshot.risks.iter().take(5) {
            text.push_str(&format!(
                "- [{}] {}: {}\n",
                risk.kind, risk.subject, risk.detail
            ));
        }
        text.push('\n');
    }

    if !snapshot.recent_commits.is_empty() {
        text.push_str("Recent commits:\n");
        for commit in snapshot.recent_commits.iter().take(5) {
            text.push_str(&format!(
                "- {} {} by {}\n",
                commit.commit, commit.subject, commit.author
            ));
        }
    }

    Ok(text.trim().to_string())
}

fn collect_snapshot_from_contexts(contexts: &[CommitContext]) -> AnalyticsSnapshot {
    collect_snapshot_from_sources(contexts, &[])
}

fn collect_snapshot_from_sources(
    contexts: &[CommitContext],
    git_activity: &[crate::git::CommitActivityRecord],
) -> AnalyticsSnapshot {
    let observed = observed_commits(contexts, git_activity);
    let total_commits = observed.len();
    let mut author_commits: HashMap<String, usize> = HashMap::new();
    let mut author_files: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut file_counts: HashMap<String, usize> = HashMap::new();
    let mut file_authors: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut weekly: BTreeMap<String, usize> = BTreeMap::new();
    let mut files_touched = std::collections::HashSet::new();
    let mut commits_last_7d = 0usize;
    let mut commits_last_30d = 0usize;
    let now = chrono::Utc::now();

    // intent_breakdown: tallied from captured context analysis data.
    let mut intent = IntentBreakdown::default();

    // file_risk: tracks the maximum impact_score seen for each file path
    // across all contexts, used to populate FileMetrics::risk_score.
    let mut file_risk: HashMap<String, u32> = HashMap::new();

    // Build a map from short-hash / full-hash → context so we can look up
    // analysis data for each observed commit without a second O(n²) scan.
    let context_by_hash: HashMap<&str, &CommitContext> = contexts
        .iter()
        .map(|ctx| (ctx.commit.as_str(), ctx))
        .collect();

    let mut recent = Vec::new();

    for commit in observed {
        *author_commits.entry(commit.author.clone()).or_insert(0) += 1;

        if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&commit.timestamp) {
            let parsed = parsed.with_timezone(&chrono::Utc);
            let age = now.signed_duration_since(parsed);
            if age <= chrono::Duration::days(7) {
                commits_last_7d += 1;
            }
            if age <= chrono::Duration::days(30) {
                commits_last_30d += 1;
            }

            let iso = parsed.iso_week();
            let week_key = format!("{}-W{:02}", iso.year(), iso.week());
            *weekly.entry(week_key).or_insert(0) += 1;
        }

        // Look up the context for this commit (matched by short or full hash).
        let matched_ctx = context_by_hash.get(commit.commit.as_str());

        // ── intent_breakdown ──────────────────────────────────────────────
        // Only count commits where a context with real analysis data exists.
        if let Some(ctx) = matched_ctx {
            if !ctx.analysis.is_empty() {
                match ctx.analysis.intent.category {
                    ChangeCategory::Feature => intent.feature += 1,
                    ChangeCategory::BugFix => intent.fix += 1,
                    ChangeCategory::Refactor => intent.refactor += 1,
                    ChangeCategory::Performance => intent.performance += 1,
                    ChangeCategory::Documentation => intent.docs += 1,
                    // DependencyUpdate | Test | Chore | Unknown → unknown bucket.
                    // There is no Security variant in ChangeCategory; security
                    // signals surface through RiskLevel on IntentClassification
                    // instead.  If a future variant is added, add a branch here.
                    _ => intent.unknown += 1,
                }

                // ── file_risk ─────────────────────────────────────────────
                // Propagate the commit-level impact_score to every file it
                // touched so we can assign a best-effort risk score per file.
                let impact = ctx.analysis.impact.impact_score;
                if impact > 0 {
                    for file in &commit.files {
                        let entry = file_risk.entry(file.clone()).or_insert(0);
                        if impact > *entry {
                            *entry = impact;
                        }
                    }
                }
            }
        }

        for file in &commit.files {
            files_touched.insert(file.clone());
            *file_counts.entry(file.clone()).or_insert(0) += 1;
            *file_authors
                .entry(file.clone())
                .or_default()
                .entry(commit.author.clone())
                .or_insert(0) += 1;
            *author_files
                .entry(commit.author.clone())
                .or_default()
                .entry(file.clone())
                .or_insert(0) += 1;
        }

        recent.push(RecentCommitMetric {
            commit: commit.commit,
            timestamp: commit.timestamp,
            author: commit.author,
            subject: commit.subject,
            files_changed: commit.files.len(),
            files: commit.files,
        });
    }

    recent.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
    recent.truncate(20);

    let mut people = author_commits
        .into_iter()
        .map(|(author, commits)| {
            let file_map = author_files.remove(&author).unwrap_or_default();
            let mut top_files = file_map
                .iter()
                .map(|(path, count)| (path.clone(), *count))
                .collect::<Vec<_>>();
            top_files.sort_by_key(|right| std::cmp::Reverse(right.1));

            PersonMetrics {
                files_touched: file_map.len(),
                top_files: top_files
                    .into_iter()
                    .take(5)
                    .map(|(path, _)| path)
                    .collect(),
                author,
                commits,
            }
        })
        .collect::<Vec<_>>();
    people.sort_by_key(|right| std::cmp::Reverse(right.commits));

    let mut files = file_counts
        .into_iter()
        .map(|(path, commits)| {
            let authors = file_authors.remove(&path).unwrap_or_default();
            let total: usize = authors.values().sum();
            let (top_author, top_author_count) = authors
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .unwrap_or_else(|| ("unknown".to_string(), 0));

            // Pull the max impact score we observed for this file, if any.
            let risk_score = file_risk.get(&path).copied();

            FileMetrics {
                path,
                commits,
                top_author: top_author.clone(),
                top_author_share: if total == 0 {
                    0.0
                } else {
                    top_author_count as f64 / total as f64
                },
                risk_score,
            }
        })
        .collect::<Vec<_>>();
    files.sort_by_key(|right| std::cmp::Reverse(right.commits));

    let mut risks = Vec::new();
    for file in files.iter().take(20) {
        if file.commits >= 3 && file.top_author_share >= 0.80 {
            risks.push(RiskMetric {
                kind: "knowledge-silo".to_string(),
                subject: file.path.clone(),
                detail: format!(
                    "{} owns {:.0}% of observed changes",
                    file.top_author,
                    file.top_author_share * 100.0
                ),
            });
        }
    }

    for commit in recent.iter().take(10) {
        if commit.files_changed >= 6 {
            risks.push(RiskMetric {
                kind: "broad-change".to_string(),
                subject: commit.commit.clone(),
                detail: format!("{} touched {} files", commit.subject, commit.files_changed),
            });
        }
    }

    let weekly_activity = weekly
        .into_iter()
        .map(|(week, commits)| WeeklyActivity { week, commits })
        .collect::<Vec<_>>();

    AnalyticsSnapshot {
        generated_at: chrono::Utc::now().to_rfc3339(),
        overview: OverviewMetrics {
            total_commits,
            unique_authors: people.len(),
            files_touched: files_touched.len(),
            commits_last_7d,
            commits_last_30d,
        },
        people,
        files,
        weekly_activity,
        risks,
        recent_commits: recent,
        intent_breakdown: intent,
    }
}

struct ObservedCommit {
    commit: String,
    timestamp: String,
    author: String,
    subject: String,
    files: Vec<String>,
}

fn observed_commits(
    contexts: &[CommitContext],
    git_activity: &[crate::git::CommitActivityRecord],
) -> Vec<ObservedCommit> {
    if git_activity.is_empty() {
        return contexts
            .iter()
            .map(|context| observed_from_context(context, None))
            .collect();
    }

    let mut observed = Vec::new();

    for record in git_activity {
        let matched = contexts
            .iter()
            .enumerate()
            .find(|(_, context)| commit_matches_context(record, context));

        observed.push(observed_from_git(
            record,
            matched.map(|(_, context)| context),
        ));
    }

    observed
}

fn observed_from_git(
    record: &crate::git::CommitActivityRecord,
    context: Option<&CommitContext>,
) -> ObservedCommit {
    let author = context
        .map(|context| author_for_context(context, Some(record)))
        .unwrap_or_else(|| record.author.clone());
    let files = context
        .map(|context| files_for_context(context, Some(record)))
        .unwrap_or_else(|| normalize_files(record.files.clone()));

    ObservedCommit {
        commit: record.short_hash.clone(),
        timestamp: record.timestamp.clone(),
        author,
        subject: record.subject.clone(),
        files,
    }
}

fn observed_from_context(
    context: &CommitContext,
    git_record: Option<&crate::git::CommitActivityRecord>,
) -> ObservedCommit {
    ObservedCommit {
        commit: context.commit.clone(),
        timestamp: git_record
            .map(|record| record.timestamp.clone())
            .unwrap_or_else(|| context.timestamp.clone()),
        author: author_for_context(context, git_record),
        subject: git_record
            .map(|record| record.subject.clone())
            .unwrap_or_else(|| crate::git::commit_subject(&context.commit).unwrap_or_default()),
        files: files_for_context(context, git_record),
    }
}

fn author_for_context(
    context: &CommitContext,
    git_record: Option<&crate::git::CommitActivityRecord>,
) -> String {
    if !context.behavior.author.trim().is_empty() {
        context.behavior.author.clone()
    } else if let Some(record) = git_record {
        record.author.clone()
    } else {
        crate::git::commit_author_name(&context.commit).unwrap_or_else(|_| "unknown".to_string())
    }
}

fn files_for_context(
    context: &CommitContext,
    git_record: Option<&crate::git::CommitActivityRecord>,
) -> Vec<String> {
    let files = if context.files.is_empty() {
        git_record
            .map(|record| record.files.clone())
            .unwrap_or_else(|| {
                crate::git::changed_files_for_commit(&context.commit).unwrap_or_default()
            })
    } else {
        context.files.clone()
    };

    normalize_files(files)
}

fn normalize_files(files: Vec<String>) -> Vec<String> {
    let mut files = files
        .into_iter()
        .map(|file| file.trim().replace('\\', "/"))
        .filter(|file| !file.is_empty())
        .collect::<Vec<_>>();
    files.sort();
    files.dedup();
    files
}

fn commit_matches_context(
    record: &crate::git::CommitActivityRecord,
    context: &CommitContext,
) -> bool {
    record.hash == context.commit
        || record.short_hash == context.commit
        || record.hash.starts_with(&context.commit)
        || context.commit.starts_with(&record.short_hash)
}

#[cfg(test)]
mod tests {
    use super::normalize_files;

    #[test]
    fn normalizes_and_deduplicates_file_paths() {
        let files = normalize_files(vec![
            "src\\main.rs".to_string(),
            " src/main.rs ".to_string(),
            String::new(),
            "README.md".to_string(),
        ]);

        assert_eq!(files, vec!["README.md", "src/main.rs"]);
    }
}
