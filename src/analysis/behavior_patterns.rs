use crate::error::AppResult;
use crate::git::AuthorCommitRecord;
use crate::storage::context::{BehaviorSnapshot, BurnoutRisk, FileExpertise};
use chrono::{DateTime, Duration, Timelike, Utc};
use std::collections::HashMap;

pub fn analyze_author_patterns(
    commit: &str,
    current_files: &[String],
) -> AppResult<BehaviorSnapshot> {
    let author = crate::git::commit_author_email(commit)
        .or_else(|_| crate::git::commit_author_name(commit))
        .unwrap_or_default();

    if author.trim().is_empty() {
        return Ok(BehaviorSnapshot::default());
    }

    let history = crate::git::author_history(&author, 120).unwrap_or_default();
    Ok(build_snapshot(&author, &history, current_files))
}

fn build_snapshot(
    author: &str,
    history: &[AuthorCommitRecord],
    current_files: &[String],
) -> BehaviorSnapshot {
    let now = Utc::now();
    let last_7d = now - Duration::days(7);
    let last_30d = now - Duration::days(30);

    let mut commits_last_7d = 0usize;
    let mut commits_last_30d = 0usize;
    let mut late_night_commits = 0usize;
    let mut hours = Vec::new();
    let mut expertise_counts: HashMap<String, usize> = HashMap::new();

    for record in history {
        let Ok(timestamp) = DateTime::parse_from_rfc3339(&record.timestamp) else {
            continue;
        };
        let timestamp = timestamp.with_timezone(&Utc);

        if timestamp >= last_30d {
            commits_last_30d += 1;
        }
        if timestamp >= last_7d {
            commits_last_7d += 1;
        }

        let hour = timestamp.hour() as u8;
        hours.push(hour);
        if !(6..22).contains(&hour) {
            late_night_commits += 1;
        }

        for path in current_files {
            if record.files.iter().any(|file| file == path) {
                *expertise_counts.entry(path.clone()).or_insert(0) += 1;
            }
        }
    }

    let late_night_ratio = percentage(late_night_commits, history.len());
    let typical_work_hours = summarize_hours(&hours);
    let burnout_risk = infer_burnout_risk(commits_last_7d, commits_last_30d, late_night_ratio);

    let mut expertise = expertise_counts
        .into_iter()
        .map(|(path, commit_count)| FileExpertise { path, commit_count })
        .collect::<Vec<_>>();
    expertise.sort_by(|left, right| {
        right
            .commit_count
            .cmp(&left.commit_count)
            .then_with(|| left.path.cmp(&right.path))
    });

    BehaviorSnapshot {
        author: author.to_string(),
        commits_last_7d,
        commits_last_30d,
        late_night_ratio,
        typical_work_hours,
        burnout_risk,
        expertise,
    }
}

fn infer_burnout_risk(
    commits_last_7d: usize,
    commits_last_30d: usize,
    late_night_ratio: u8,
) -> BurnoutRisk {
    let weekly_baseline = commits_last_30d / 4;

    if (commits_last_7d >= weekly_baseline.saturating_mul(2).max(10) && late_night_ratio >= 30)
        || late_night_ratio >= 45
    {
        BurnoutRisk::Elevated
    } else if commits_last_7d > weekly_baseline.saturating_add(4) || late_night_ratio >= 20 {
        BurnoutRisk::Watch
    } else {
        BurnoutRisk::Normal
    }
}

fn summarize_hours(hours: &[u8]) -> String {
    if hours.is_empty() {
        return String::new();
    }

    let min = *hours.iter().min().unwrap_or(&0);
    let max = *hours.iter().max().unwrap_or(&0);
    format!("{min:02}:00-{max:02}:59")
}

fn percentage(part: usize, total: usize) -> u8 {
    if total == 0 {
        0
    } else {
        ((part as f32 / total as f32) * 100.0).round() as u8
    }
}

#[cfg(test)]
mod tests {
    use super::build_snapshot;
    use crate::git::AuthorCommitRecord;
    use crate::storage::context::BurnoutRisk;

    #[test]
    fn flags_late_night_bursts() {
        let history = vec![
            AuthorCommitRecord {
                timestamp: "2026-04-08T23:10:00Z".to_string(),
                files: vec!["src/auth.rs".to_string()],
            },
            AuthorCommitRecord {
                timestamp: "2026-04-07T23:45:00Z".to_string(),
                files: vec!["src/auth.rs".to_string()],
            },
            AuthorCommitRecord {
                timestamp: "2026-04-06T22:30:00Z".to_string(),
                files: vec!["src/auth.rs".to_string()],
            },
        ];

        let snapshot = build_snapshot("alice@example.com", &history, &["src/auth.rs".to_string()]);
        assert_eq!(snapshot.burnout_risk, BurnoutRisk::Elevated);
        assert_eq!(snapshot.expertise[0].path, "src/auth.rs");
    }
}
