pub mod exporter;

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
}

pub fn collect_snapshot() -> crate::error::AppResult<AnalyticsSnapshot> {
    let contexts = crate::storage::load::load_all_contexts()?;
    Ok(collect_snapshot_from_contexts(&contexts))
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
    let mut author_commits: HashMap<String, usize> = HashMap::new();
    let mut author_files: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut file_counts: HashMap<String, usize> = HashMap::new();
    let mut file_authors: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut weekly: BTreeMap<String, usize> = BTreeMap::new();
    let mut files_touched = std::collections::HashSet::new();
    let mut commits_last_7d = 0usize;
    let mut commits_last_30d = 0usize;
    let now = chrono::Utc::now();

    let mut recent = Vec::new();

    for context in contexts {
        let author = author_for_context(context);
        *author_commits.entry(author.clone()).or_insert(0) += 1;

        if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&context.timestamp) {
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

        for file in &context.files {
            files_touched.insert(file.clone());
            *file_counts.entry(file.clone()).or_insert(0) += 1;
            *file_authors
                .entry(file.clone())
                .or_default()
                .entry(author.clone())
                .or_insert(0) += 1;
            *author_files
                .entry(author.clone())
                .or_default()
                .entry(file.clone())
                .or_insert(0) += 1;
        }

        recent.push(RecentCommitMetric {
            commit: context.commit.clone(),
            timestamp: context.timestamp.clone(),
            author,
            subject: crate::git::commit_subject(&context.commit).unwrap_or_default(),
            files_changed: context.files.len(),
        });
    }

    recent.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
    recent.truncate(10);

    let mut people = author_commits
        .into_iter()
        .map(|(author, commits)| {
            let file_map = author_files.remove(&author).unwrap_or_default();
            let mut top_files = file_map
                .iter()
                .map(|(path, count)| (path.clone(), *count))
                .collect::<Vec<_>>();
            top_files.sort_by(|left, right| right.1.cmp(&left.1));

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
    people.sort_by(|left, right| right.commits.cmp(&left.commits));

    let mut files = file_counts
        .into_iter()
        .map(|(path, commits)| {
            let authors = file_authors.remove(&path).unwrap_or_default();
            let total: usize = authors.values().sum();
            let (top_author, top_author_count) = authors
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .unwrap_or_else(|| ("unknown".to_string(), 0));

            FileMetrics {
                path,
                commits,
                top_author: top_author.clone(),
                top_author_share: if total == 0 {
                    0.0
                } else {
                    top_author_count as f64 / total as f64
                },
            }
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| right.commits.cmp(&left.commits));

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
            total_commits: contexts.len(),
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
    }
}

fn author_for_context(context: &CommitContext) -> String {
    if !context.behavior.author.trim().is_empty() {
        context.behavior.author.clone()
    } else {
        crate::git::commit_author_name(&context.commit).unwrap_or_else(|_| "unknown".to_string())
    }
}
