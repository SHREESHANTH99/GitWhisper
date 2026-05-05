use crate::ai::AiBackend;
use crate::config::AiProvider;
use crate::error::{AppError, AppResult};
use crate::history::HistoryEntry;
use crate::storage::context::CommitContext;
use reqwest::blocking::Client;
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct RelatedCommit {
    file: String,
    short_hash: String,
    subject: String,
}

#[derive(Debug, Clone)]
pub struct RelatedHistoryEntry {
    pub file: String,
    pub short_hash: String,
    pub subject: String,
}

#[derive(Debug, Clone)]
struct GeneratedSummary {
    summary: String,
    source: String,
    ai_model: Option<String>,
}

#[derive(Debug, Clone)]
struct AnnotationOutcome {
    commit: String,
    note_written: bool,
    webhook_sent: bool,
    summary: String,
    source: String,
    ai_model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CommitReport {
    pub commit: String,
    pub full_commit: String,
    pub branch: String,
    pub subject: String,
    pub summary: String,
    pub source: String,
    pub ai_model: Option<String>,
    pub changed_files: Vec<String>,
    pub note: String,
    pub risk: Option<String>,
    pub impact: Option<String>,
    pub review_summary: Option<String>,
    pub related_history: Vec<RelatedHistoryEntry>,
}

pub fn run_post_commit(api_key: &str) {
    if let Err(error) = crate::capture::capture_head_context() {
        eprintln!("{error}");
        return;
    }

    let config = match crate::config::AppConfig::load() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Failed to load .gitwhisper.toml: {error}");
            return;
        }
    };

    if !config.collaboration.auto_annotate_commits {
        log_annotation_event("post-commit", "HEAD", "capture-only", None, None, None);
        return;
    }

    if let Err(error) = annotate_commit_inner(Some("HEAD"), api_key, true) {
        eprintln!("{error}");
    }
}

pub fn annotate_commit(commit: Option<&str>, api_key: &str) {
    match annotate_commit_inner(commit, api_key, false) {
        Ok(outcome) => {
            println!("Annotated commit {}.", outcome.commit);
            println!("Summary: {}", outcome.summary);

            if outcome.note_written {
                println!("Stored Git note.");
            }

            if outcome.webhook_sent {
                println!("Sent webhook notification.");
            }

            if let Some(model) = outcome.ai_model {
                println!("Source: {} ({})", outcome.source, model);
            } else {
                println!("Source: {}", outcome.source);
            }
        }
        Err(error) => {
            eprintln!("{error}");
        }
    }
}

pub fn prepare_commit_report(
    commit: Option<&str>,
    api_key: &str,
    skip_capture: bool,
) -> AppResult<CommitReport> {
    let config = crate::config::AppConfig::load()?;
    let head_commit = crate::git::head_commit_hash()?;

    let resolved_commit = match commit {
        Some(commitish) if !commitish.trim().is_empty() && commitish != "HEAD" => {
            crate::git::resolve_commit(commitish)?
        }
        _ => head_commit.clone(),
    };

    if !skip_capture && resolved_commit == head_commit {
        let _ = crate::capture::capture_head_context();
    }

    let short_commit = crate::git::short_commit_hash_of(&resolved_commit)?;
    let branch = crate::git::current_branch().unwrap_or_else(|_| "unknown".to_string());
    let commit_message = crate::git::commit_message(&resolved_commit).unwrap_or_else(|_| {
        crate::git::CommitMessage {
            subject: String::new(),
            body: String::new(),
        }
    });
    let changed_files = crate::git::changed_files_for_commit(&resolved_commit).unwrap_or_default();
    let context = crate::storage::load::load_context(&short_commit).ok();
    let related_history = collect_related_history(
        &changed_files,
        &resolved_commit,
        config.ai.history_depth.max(3).min(8),
    );

    let generated = generate_commit_summary(
        &config,
        &short_commit,
        &commit_message,
        &changed_files,
        context.as_ref(),
        &related_history,
        api_key,
    );

    let note = format_git_note(
        &short_commit,
        &commit_message.subject,
        &changed_files,
        context.as_ref(),
        &related_history,
        &generated,
    );

    let related_entries = related_history
        .iter()
        .map(|entry| RelatedHistoryEntry {
            file: entry.file.clone(),
            short_hash: entry.short_hash.clone(),
            subject: entry.subject.clone(),
        })
        .collect::<Vec<_>>();

    let risk = context
        .as_ref()
        .filter(|ctx| !ctx.analysis.is_empty())
        .map(|ctx| ctx.analysis.intent.risk.to_string());
    let impact = context.as_ref().and_then(|ctx| {
        ctx.analysis
            .impact
            .summary()
            .or_else(|| ctx.analysis.diff.summary())
    });
    let review_summary = context.as_ref().and_then(|ctx| ctx.review.summary());

    Ok(CommitReport {
        commit: short_commit,
        full_commit: resolved_commit,
        branch,
        subject: commit_message.subject,
        summary: generated.summary,
        source: generated.source,
        ai_model: generated.ai_model,
        changed_files,
        note,
        risk,
        impact,
        review_summary,
        related_history: related_entries,
    })
}

fn annotate_commit_inner(
    commit: Option<&str>,
    api_key: &str,
    skip_capture: bool,
) -> AppResult<AnnotationOutcome> {
    let config = crate::config::AppConfig::load()?;
    let report = prepare_commit_report(commit, api_key, skip_capture)?;

    let note_written = if config.collaboration.enable_git_notes {
        crate::git::add_git_note(
            &report.full_commit,
            &config.collaboration.git_notes_ref,
            &report.note,
        )?;
        true
    } else {
        false
    };

    let webhook_sent = match send_webhook(
        &config,
        &report.full_commit,
        &report.commit,
        &report.subject,
        &report.changed_files,
        &report.summary,
        report.ai_model.as_deref(),
        report.source.as_str(),
        &report.note,
    ) {
        Ok(sent) => sent,
        Err(error) => {
            let error_text = error.to_string();
            log_annotation_event(
                "webhook-error",
                &report.commit,
                report.source.as_str(),
                report.ai_model.as_deref(),
                Some(error_text.as_str()),
                None,
            );
            false
        }
    };

    let auto_delivery_error = crate::integrations::auto_deliver_commit_report(&config, &report);
    if let Err(error) = auto_delivery_error {
        let error_text = error.to_string();
        log_annotation_event(
            "auto-delivery-error",
            &report.commit,
            report.source.as_str(),
            report.ai_model.as_deref(),
            Some(error_text.as_str()),
            None,
        );
    }

    log_annotation_event(
        "annotate",
        &report.commit,
        report.source.as_str(),
        report.ai_model.as_deref(),
        None,
        None,
    );

    Ok(AnnotationOutcome {
        commit: report.commit,
        note_written,
        webhook_sent,
        summary: report.summary,
        source: report.source,
        ai_model: report.ai_model,
    })
}

fn generate_commit_summary(
    config: &crate::config::AppConfig,
    short_commit: &str,
    commit_message: &crate::git::CommitMessage,
    changed_files: &[String],
    context: Option<&CommitContext>,
    related_history: &[RelatedCommit],
    api_key: &str,
) -> GeneratedSummary {
    let prompt = build_annotation_prompt(
        short_commit,
        commit_message,
        changed_files,
        context,
        related_history,
    );
    let cloud_model = if config.ai.model.trim().is_empty() {
        "gemini-1.5-flash"
    } else {
        config.ai.model.as_str()
    };
    let local_model = if config.ai.local_model.trim().is_empty() {
        "mistral"
    } else {
        config.ai.local_model.as_str()
    };
    let has_api_key = !api_key.trim().is_empty();
    let offline_mode = config.privacy.offline_mode;

    let mut backends =
        crate::ai::model_selector::choose_backends(&config.ai, &prompt, has_api_key, offline_mode);

    if backends.contains(&AiBackend::Local)
        && !crate::ai::local_ollama::is_available(&config.ai.ollama_url, 2)
    {
        backends.retain(|backend| *backend != AiBackend::Local);
    }

    if backends.is_empty() {
        return GeneratedSummary {
            summary: heuristic_summary(commit_message, changed_files, context, related_history),
            source: "heuristic".to_string(),
            ai_model: None,
            // Source-specific fallback reason is intentionally not surfaced in the final note text.
            // The collaboration flow only needs the resulting summary + source label.
        };
    }

    let mut last_model_key = None::<String>;

    for backend in backends {
        let started = Instant::now();
        let model_key = match backend {
            AiBackend::Cloud => cloud_model.to_string(),
            AiBackend::Local => format!("local:{local_model}"),
        };

        let result = match backend {
            AiBackend::Cloud => crate::ai::cloud_gemini::generate(
                &prompt,
                cloud_model,
                api_key,
                config.ai.request_timeout_secs,
            ),
            AiBackend::Local => crate::ai::local_ollama::generate(
                &prompt,
                local_model,
                &config.ai.ollama_url,
                config.ai.request_timeout_secs,
            ),
        };
        let _elapsed = started.elapsed().as_secs_f64();

        match result {
            Ok(text) => {
                return GeneratedSummary {
                    summary: normalize_summary(&text),
                    source: "ai".to_string(),
                    ai_model: Some(model_key),
                };
            }
            Err(_error) => {
                last_model_key = Some(model_key);
            }
        }
    }

    let _fallback_reason = match config.ai.provider {
        AiProvider::Cloud => {
            if offline_mode {
                "offline_mode_enabled".to_string()
            } else {
                "missing_api_key".to_string()
            }
        }
        AiProvider::Local => "local_ai_unavailable".to_string(),
        AiProvider::Hybrid => "no_ai_backend_available".to_string(),
    };

    GeneratedSummary {
        summary: heuristic_summary(commit_message, changed_files, context, related_history),
        source: "heuristic".to_string(),
        ai_model: last_model_key,
    }
}

fn build_annotation_prompt(
    short_commit: &str,
    commit_message: &crate::git::CommitMessage,
    changed_files: &[String],
    context: Option<&CommitContext>,
    related_history: &[RelatedCommit],
) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are generating a compact Git note for teammates about a single commit.\n");
    prompt
        .push_str("Focus on what changed, why it likely changed, and the most relevant impact.\n");
    prompt.push_str("Only make claims supported by the evidence. If uncertain, say likely.\n");
    prompt
        .push_str("Return only a concise 1-2 sentence summary with no bullets and no heading.\n\n");
    prompt.push_str(&format!("Commit: {short_commit}\n"));
    prompt.push_str(&format!("Subject: {}\n", commit_message.subject));

    if !commit_message.body.trim().is_empty() {
        prompt.push_str(&format!(
            "Message details: {}\n",
            compact_text(&commit_message.body, 260)
        ));
    }

    if !changed_files.is_empty() {
        prompt.push_str(&format!(
            "Files changed: {}\n",
            format_list(changed_files, 8)
        ));
    }

    if let Some(context) = context {
        if !context.analysis.is_empty() {
            prompt.push_str(&format!(
                "Detected intent: {}\n",
                context.analysis.intent.summary()
            ));
            if let Some(summary) = context.analysis.diff.summary() {
                prompt.push_str(&format!("Diff summary: {summary}\n"));
            }
            if let Some(summary) = context.analysis.diff.semantic_summary() {
                prompt.push_str(&format!("Semantic diff: {}\n", compact_text(&summary, 220)));
            }
            if let Some(summary) = context.analysis.impact.summary() {
                prompt.push_str(&format!("Impact summary: {summary}\n"));
            }
        }

        if let Some(summary) = context.review.summary() {
            prompt.push_str(&format!("Review/test context: {summary}\n"));
        }

        if !context.commands.is_empty() {
            prompt.push_str(&format!(
                "Developer commands: {}\n",
                format_list(&context.commands, 5)
            ));
        }
    }

    if !related_history.is_empty() {
        prompt.push_str("Recent related history:\n");
        for entry in related_history.iter().take(6) {
            prompt.push_str(&format!(
                "- {} {} ({})\n",
                entry.short_hash, entry.subject, entry.file
            ));
        }
    }

    prompt
}

fn heuristic_summary(
    commit_message: &crate::git::CommitMessage,
    changed_files: &[String],
    context: Option<&CommitContext>,
    related_history: &[RelatedCommit],
) -> String {
    let mut parts = Vec::new();

    if !commit_message.subject.trim().is_empty() {
        parts.push(format!(
            "This commit appears to {}.",
            commit_message.subject.trim().to_lowercase()
        ));
    } else {
        parts.push("This commit updates the codebase.".to_string());
    }

    if let Some(context) = context {
        if !context.analysis.is_empty() {
            parts.push(format!(
                "The captured intent looks like a {} change with {} urgency and {} risk.",
                context.analysis.intent.category,
                context.analysis.intent.urgency,
                context.analysis.intent.risk
            ));
        }

        if let Some(summary) = context.analysis.impact.summary() {
            parts.push(format!("Impact appears to be {}.", summary));
        } else if let Some(summary) = context.analysis.diff.summary() {
            parts.push(format!("The change footprint was {}.", summary));
        }
    } else if !changed_files.is_empty() {
        parts.push(format!("It touched {}.", format_list(changed_files, 4)));
    }

    if let Some(previous) = related_history.first() {
        parts.push(format!(
            "It likely builds on earlier work such as `{}`.",
            previous.subject
        ));
    }

    normalize_summary(&parts.join(" "))
}

fn collect_related_history(
    changed_files: &[String],
    current_commit: &str,
    per_file_limit: usize,
) -> Vec<RelatedCommit> {
    let mut seen = HashSet::new();
    let mut related = Vec::new();

    for file in changed_files.iter().take(5) {
        let Ok(history) = crate::history::load_history_for_file(file, per_file_limit) else {
            continue;
        };

        collect_file_history(file, &history, current_commit, &mut seen, &mut related);
    }

    related
}

fn collect_file_history(
    file: &str,
    history: &[HistoryEntry],
    current_commit: &str,
    seen: &mut HashSet<String>,
    related: &mut Vec<RelatedCommit>,
) {
    for entry in history {
        if entry.commit.hash == current_commit {
            continue;
        }

        if seen.insert(entry.commit.hash.clone()) {
            related.push(RelatedCommit {
                file: file.to_string(),
                short_hash: entry.commit.short_hash.clone(),
                subject: entry.commit.subject.clone(),
            });
        }

        if related.len() >= 8 {
            break;
        }
    }
}

fn format_git_note(
    short_commit: &str,
    subject: &str,
    changed_files: &[String],
    context: Option<&CommitContext>,
    related_history: &[RelatedCommit],
    generated: &GeneratedSummary,
) -> String {
    let mut lines = vec![
        "[gitwhisper-explanation]".to_string(),
        format!("Commit: {short_commit}"),
        format!("Summary: {}", generated.summary),
    ];

    if !subject.trim().is_empty() {
        lines.push(format!("Subject: {}", compact_text(subject, 160)));
    }

    if !changed_files.is_empty() {
        lines.push(format!("Files changed: {}", format_list(changed_files, 8)));
    }

    if let Some(context) = context {
        if !context.analysis.is_empty() {
            lines.push(format!(
                "Type: {}",
                compact_text(&context.analysis.intent.summary(), 180)
            ));

            if let Some(summary) = context.analysis.impact.summary() {
                lines.push(format!("Impact: {}", compact_text(&summary, 180)));
            } else if let Some(summary) = context.analysis.diff.summary() {
                lines.push(format!("Impact: {}", compact_text(&summary, 180)));
            }
        }

        if let Some(summary) = context.review.summary() {
            lines.push(format!("Reviewed/Tested: {}", compact_text(&summary, 180)));
        }
    }

    if !related_history.is_empty() {
        let related = related_history
            .iter()
            .take(3)
            .map(|entry| format!("{} {}", entry.short_hash, entry.subject))
            .collect::<Vec<_>>();
        lines.push(format!("Related history: {}", related.join("; ")));
    }

    let source = match &generated.ai_model {
        Some(model) => format!("{} ({model})", generated.source),
        None => generated.source.clone(),
    };
    lines.push(format!("Generated by: gitwhisper {source}"));
    lines.push(format!("Generated at: {}", chrono::Utc::now().to_rfc3339()));

    lines.join("\n")
}

fn send_webhook(
    config: &crate::config::AppConfig,
    commit: &str,
    short_commit: &str,
    subject: &str,
    changed_files: &[String],
    summary: &str,
    ai_model: Option<&str>,
    source: &str,
    note: &str,
) -> AppResult<bool> {
    if config.collaboration.webhook_url.trim().is_empty() {
        return Ok(false);
    }

    let payload = json!({
        "event": "gitwhisper.commit_annotated",
        "commit": commit,
        "short_commit": short_commit,
        "subject": subject,
        "files": changed_files,
        "summary": summary,
        "note": note,
        "source": source,
        "ai_model": ai_model,
        "generated_at": chrono::Utc::now().to_rfc3339(),
    });

    let client = Client::builder()
        .timeout(Duration::from_secs(
            config.collaboration.webhook_timeout_secs.max(1),
        ))
        .build()?;

    let response = client
        .post(&config.collaboration.webhook_url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()?;

    if response.status().is_success() {
        Ok(true)
    } else {
        Err(AppError::message(format!(
            "Webhook request failed with status {}.",
            response.status()
        )))
    }
}

fn log_annotation_event(
    event: &str,
    commit: &str,
    source: &str,
    ai_model: Option<&str>,
    error: Option<&str>,
    elapsed_secs: Option<f64>,
) {
    let Ok(log_dir) = crate::storage::log_dir() else {
        return;
    };
    if fs::create_dir_all(&log_dir).is_err() {
        return;
    }

    let Ok(log_path) = crate::storage::collaboration_log_path() else {
        return;
    };

    let mut entry = format!(
        "{}\nevent: {}\ncommit: {}\nsource: {}\n",
        chrono::Utc::now().to_rfc3339(),
        event,
        commit,
        source
    );

    if let Some(ai_model) = ai_model {
        entry.push_str(&format!("ai_model: {}\n", ai_model));
    }

    if let Some(elapsed_secs) = elapsed_secs {
        entry.push_str(&format!("elapsed_secs: {:.3}\n", elapsed_secs));
    }

    if let Some(error) = error {
        entry.push_str(&format!("error: {}\n", error));
    }

    entry.push('\n');

    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .and_then(|mut handle| handle.write_all(entry.as_bytes()));
}

fn normalize_summary(input: &str) -> String {
    let trimmed = input.trim();
    let stripped = if trimmed.to_ascii_lowercase().starts_with("summary:") {
        trimmed[8..].trim()
    } else {
        trimmed
    };

    compact_text(stripped, 420)
}

fn format_list(values: &[String], limit: usize) -> String {
    let visible = values
        .iter()
        .take(limit)
        .map(|value| compact_text(value, 60))
        .collect::<Vec<_>>();

    if values.len() > limit {
        format!("{} (+{} more)", visible.join(", "), values.len() - limit)
    } else {
        visible.join(", ")
    }
}

fn compact_text(input: &str, max_len: usize) -> String {
    let collapsed = input.split_whitespace().collect::<Vec<_>>().join(" ");
    let collapsed_len = collapsed.chars().count();
    if collapsed_len <= max_len {
        collapsed
    } else if max_len <= 3 {
        ".".repeat(max_len)
    } else {
        let prefix = collapsed.chars().take(max_len - 3).collect::<String>();
        format!("{prefix}...")
    }
}

#[cfg(test)]
mod tests {
    use super::{compact_text, normalize_summary};

    #[test]
    fn strips_summary_prefix_from_ai_output() {
        assert_eq!(
            normalize_summary("Summary: Added commit annotation support."),
            "Added commit annotation support."
        );
    }

    #[test]
    fn compacts_whitespace_for_note_text() {
        assert_eq!(
            compact_text("hello   world\nfrom\tgitwhisper", 50),
            "hello world from gitwhisper"
        );
    }
}
