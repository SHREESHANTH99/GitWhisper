use crate::error::{AppError, AppResult};
use crate::metrics::AnalyticsSnapshot;
use std::fs;
use std::path::Path;

pub fn export_snapshot(snapshot: &AnalyticsSnapshot, format: &str, output: &Path) -> AppResult<()> {
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    match format {
        "json" => {
            let raw = serde_json::to_string_pretty(snapshot)?;
            fs::write(output, raw)?;
            Ok(())
        }
        "csv" => {
            fs::write(output, snapshot_to_csv(snapshot))?;
            Ok(())
        }
        other => Err(AppError::message(format!(
            "Unsupported export format `{other}`. Use `json` or `csv`."
        ))),
    }
}

pub fn snapshot_to_csv(snapshot: &AnalyticsSnapshot) -> String {
    let mut lines = Vec::new();
    lines.push("record_type,key,value,extra1,extra2".to_string());
    lines.push(format!(
        "overview,total_commits,{},,",
        snapshot.overview.total_commits
    ));
    lines.push(format!(
        "overview,unique_authors,{},,",
        snapshot.overview.unique_authors
    ));
    lines.push(format!(
        "overview,files_touched,{},,",
        snapshot.overview.files_touched
    ));

    let ib = &snapshot.intent_breakdown;
    for (label, count) in &[
        ("feature", ib.feature),
        ("fix", ib.fix),
        ("refactor", ib.refactor),
        ("security", ib.security),
        ("performance", ib.performance),
        ("docs", ib.docs),
        ("unknown", ib.unknown),
    ] {
        lines.push(format!("intent,{},{},,", label, count));
    }

    for person in &snapshot.people {
        lines.push(format!(
            "person,{},{},{},{}",
            csv_escape(&person.author),
            person.commits,
            person.files_touched,
            csv_escape(&person.top_files.join(" | "))
        ));
    }

    for file in &snapshot.files {
        let risk = file.risk_score.map(|s| s.to_string()).unwrap_or_default();
        lines.push(format!(
            "file,{},{},{} ({:.0}%),{}",
            csv_escape(&file.path),
            file.commits,
            csv_escape(&file.top_author),
            file.top_author_share * 100.0,
            risk
        ));
    }

    for week in &snapshot.weekly_activity {
        lines.push(format!("week,{},{},,", week.week, week.commits));
    }

    for risk in &snapshot.risks {
        lines.push(format!(
            "risk,{},{},{},",
            csv_escape(&risk.kind),
            csv_escape(&risk.subject),
            csv_escape(&risk.detail)
        ));
    }

    for commit in &snapshot.recent_commits {
        lines.push(format!(
            "commit,{},{},{},",
            csv_escape(&commit.commit),
            commit.files_changed,
            csv_escape(&format!(
                "{} by {} | {}",
                commit.subject,
                commit.author,
                commit.files.join(" | ")
            ))
        ));
    }

    lines.join("\n")
}

fn csv_escape(value: &str) -> String {
    let escaped = value.replace('"', "\"\"");
    format!("\"{escaped}\"")
}
