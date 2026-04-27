use crate::auth::{ensure_permission, Permission};
use crate::db::{AppDatabase, Database, FeedbackRecord};
use crate::error::{AppError, AppResult};

pub fn submit_feedback(
    commit: &str,
    rating: i32,
    feedback: &str,
    tags: &[String],
) -> AppResult<FeedbackRecord> {
    let config = crate::config::AppConfig::load()?;
    if !config.feedback.enabled {
        return Err(AppError::message("Feedback is disabled in configuration."));
    }

    let user = ensure_permission(&config, Permission::SubmitFeedback)?;
    let resolved_commit = crate::git::resolve_commit(commit)?;
    let short_commit = crate::git::short_commit_hash_of(&resolved_commit)?;
    let record = FeedbackRecord {
        id: format!("{}-{}", short_commit, chrono::Utc::now().timestamp_millis()),
        timestamp: chrono::Utc::now().to_rfc3339(),
        commit: short_commit.clone(),
        actor: user.username.clone(),
        rating,
        feedback: feedback.trim().to_string(),
        tags: tags.to_vec(),
    };

    let db = Database::open(&config)?;
    db.append_feedback(&record)?;
    crate::audit::record(
        &config,
        &user.username,
        "feedback.submit",
        &short_commit,
        "success",
        serde_json::json!({
            "rating": rating,
            "tags": tags,
        }),
    )?;
    Ok(record)
}

pub fn recent_feedback(limit: usize) -> AppResult<Vec<FeedbackRecord>> {
    let config = crate::config::AppConfig::load()?;
    let _ = ensure_permission(&config, Permission::ViewReports)?;
    let db = Database::open(&config)?;
    db.list_feedback(limit.max(1))
}

pub fn export_feedback(output: &str, format: &str) -> AppResult<()> {
    let config = crate::config::AppConfig::load()?;
    let _ = ensure_permission(&config, Permission::ViewReports)?;
    let db = Database::open(&config)?;
    let records = db.list_all_feedback()?;

    let path = std::path::Path::new(output);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    match format {
        "json" => {
            let raw = serde_json::to_string_pretty(&records)?;
            std::fs::write(path, raw)?;
        }
        "csv" => {
            let mut lines = vec!["id,timestamp,commit,actor,rating,feedback,tags".to_string()];
            for record in records {
                lines.push(format!(
                    "{},{},{},{},{},\"{}\",\"{}\"",
                    csv_escape(&record.id),
                    csv_escape(&record.timestamp),
                    csv_escape(&record.commit),
                    csv_escape(&record.actor),
                    record.rating,
                    record.feedback.replace('"', "\"\""),
                    record.tags.join("|").replace('"', "\"\"")
                ));
            }
            std::fs::write(path, lines.join("\n"))?;
        }
        other => {
            return Err(AppError::message(format!(
                "Unsupported feedback export format `{other}`. Use `json` or `csv`."
            )))
        }
    }

    Ok(())
}

pub fn show_feedback(commit: &str, good: bool, poor: bool, corrected: &str, tags: &str) {
    let rating = if good {
        5
    } else if poor {
        1
    } else {
        3
    };
    let note = if corrected.trim().is_empty() {
        if good {
            "Marked as helpful.".to_string()
        } else if poor {
            "Marked as poor.".to_string()
        } else {
            "Recorded neutral feedback.".to_string()
        }
    } else {
        corrected.trim().to_string()
    };
    let parsed_tags = parse_tags(tags);

    match submit_feedback(commit, rating, &note, &parsed_tags) {
        Ok(record) => {
            println!(
                "Stored feedback for commit {} by {} (rating {}).",
                record.commit, record.actor, record.rating
            );
            if !record.feedback.is_empty() {
                println!("Note: {}", record.feedback);
            }
            if !record.tags.is_empty() {
                println!("Tags: {}", record.tags.join(", "));
            }
        }
        Err(error) => eprintln!("{error}"),
    }
}

pub fn show_recent_feedback(limit: usize) {
    match recent_feedback(limit) {
        Ok(records) if records.is_empty() => println!("No feedback records found yet."),
        Ok(records) => {
            println!("Recent feedback:\n");
            for record in records {
                println!(
                    "{}  {}  rating {}  {}",
                    record.commit, record.timestamp, record.rating, record.actor
                );
                if !record.feedback.is_empty() {
                    println!("  {}", record.feedback);
                }
                if !record.tags.is_empty() {
                    println!("  Tags: {}", record.tags.join(", "));
                }
                println!();
            }
        }
        Err(error) => eprintln!("{error}"),
    }
}

pub fn show_feedback_export(output: &str, format: &str) {
    match export_feedback(output, format) {
        Ok(()) => println!("Exported feedback to {}", output),
        Err(error) => eprintln!("{error}"),
    }
}

fn parse_tags(tags: &str) -> Vec<String> {
    tags.split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect()
}

fn csv_escape(value: &str) -> String {
    value.replace('"', "\"\"")
}
