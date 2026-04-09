use crate::analysis::ChangeCategory;
use crate::git::FileCommit;
use crate::storage::context::CommitContext;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct HistoryEntry {
    commit: FileCommit,
    context: Option<CommitContext>,
}

#[derive(Debug, Deserialize)]
struct CachedExplanation {
    source: String,
    ai_model: String,
    last_commit_hash: String,
    commit_history_hash: String,
    explanation: String,
}

pub fn explain_file(file: &str, api_key: &str) {
    let normalized_file = match crate::git::normalize_repo_path(file) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    let config = match crate::config::AppConfig::load() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Failed to load .gitwhisper.toml: {error}");
            return;
        }
    };

    let history_limit = config.ai.history_depth.max(1);
    let ai_model = if config.ai.model.trim().is_empty() {
        "gemini-1.5-flash"
    } else {
        config.ai.model.as_str()
    };

    let history = match load_history_for_file(&normalized_file, history_limit) {
        Ok(history) => history,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    if history.is_empty() {
        println!("No Git history found for {}.", normalized_file);
        return;
    }

    let historical_contexts = history
        .iter()
        .filter_map(|entry| entry.context.clone())
        .collect::<Vec<_>>();
    let predicted_files = crate::storage::predictive_cache::predict_related_files(
        &normalized_file,
        history[0].context.as_ref(),
        &historical_contexts,
        5,
    );

    let latest_commit_hash = history[0].commit.hash.clone();
    let history_hash = history_fingerprint(&history);
    let ai_enabled = !api_key.is_empty() && !config.privacy.offline_mode;

    if let Some(cache) = try_cache_lookup(
        &normalized_file,
        &latest_commit_hash,
        &history_hash,
        ai_enabled,
        ai_model,
    ) {
        let heading = if cache.source == "ai" {
            "AI Explanation"
        } else {
            "Explanation"
        };
        println!("Using cached {} for {}\n", cache.source, normalized_file);
        println!("{heading}:\n{}", cache.explanation);
        print_related_files(&predicted_files);
        log_ai_event(&normalized_file, true, None, None, &cache.source);
        return;
    }

    let fallback = heuristic_explanation(&normalized_file, &history, &predicted_files);
    if !ai_enabled {
        println!("Explanation:\n{}", fallback);
        save_cache(
            &normalized_file,
            &latest_commit_hash,
            &history_hash,
            &fallback,
            "heuristic",
            ai_model,
            &predicted_files,
        );
        print_related_files(&predicted_files);
        let reason = if config.privacy.offline_mode {
            "offline_mode_enabled"
        } else {
            "missing_api_key"
        };
        log_ai_event(&normalized_file, false, None, Some(reason), "heuristic");
        return;
    }

    println!("Generating AI explanation for {}...\n", normalized_file);
    let prompt = build_prompt(&normalized_file, &history, &predicted_files);
    let client = match Client::builder()
        .timeout(Duration::from_secs(config.ai.request_timeout_secs.max(1)))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Failed to create HTTP client: {}", error);
            println!("Explanation:\n{}", fallback);
            return;
        }
    };

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        ai_model, api_key
    );

    let started = Instant::now();
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }]
        }))
        .send();
    let elapsed = started.elapsed().as_secs_f64();

    match response {
        Ok(response) => {
            let status = response.status();
            let json_response = response
                .json::<serde_json::Value>()
                .unwrap_or_else(|_| json!({}));

            if !status.is_success() {
                let message = json_response
                    .get("error")
                    .and_then(|value| value.get("message"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("Gemini request failed");
                eprintln!("AI request failed: {}", message);
                println!("Explanation:\n{}", fallback);
                save_cache(
                    &normalized_file,
                    &latest_commit_hash,
                    &history_hash,
                    &fallback,
                    "heuristic",
                    ai_model,
                    &predicted_files,
                );
                log_ai_event(
                    &normalized_file,
                    false,
                    Some(elapsed),
                    Some(message),
                    "heuristic",
                );
                return;
            }

            if let Some(text) = extract_text(&json_response) {
                println!("AI Explanation:\n{}", text);
                print_related_files(&predicted_files);
                save_cache(
                    &normalized_file,
                    &latest_commit_hash,
                    &history_hash,
                    text,
                    "ai",
                    ai_model,
                    &predicted_files,
                );
                log_ai_event(&normalized_file, false, Some(elapsed), None, "ai");
            } else {
                eprintln!("AI response did not include explanation text.");
                println!("Explanation:\n{}", fallback);
                save_cache(
                    &normalized_file,
                    &latest_commit_hash,
                    &history_hash,
                    &fallback,
                    "heuristic",
                    ai_model,
                    &predicted_files,
                );
                log_ai_event(
                    &normalized_file,
                    false,
                    Some(elapsed),
                    Some("missing_candidate_text"),
                    "heuristic",
                );
            }
        }
        Err(error) => {
            eprintln!("AI request failed: {}", error);
            println!("Explanation:\n{}", fallback);
            save_cache(
                &normalized_file,
                &latest_commit_hash,
                &history_hash,
                &fallback,
                "heuristic",
                ai_model,
                &predicted_files,
            );
            log_ai_event(
                &normalized_file,
                false,
                Some(elapsed),
                Some(&error.to_string()),
                "heuristic",
            );
        }
    }
}

fn load_history_for_file(
    file: &str,
    limit: usize,
) -> Result<Vec<HistoryEntry>, crate::error::AppError> {
    let git_history = crate::git::file_history(file, limit)?;
    let entries = git_history
        .into_iter()
        .map(|commit| {
            let context = crate::storage::load::load_context(&commit.short_hash).ok();
            HistoryEntry { commit, context }
        })
        .collect();

    Ok(entries)
}

fn build_prompt(file: &str, history: &[HistoryEntry], predicted_files: &[String]) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are helping a developer understand why a file changed over time.\n");
    prompt.push_str("Focus on intent, problem solved, and the evolution of the file.\n");
    prompt.push_str("Only make claims supported by the evidence. If something is uncertain, say it likely happened rather than stating it as fact.\n\n");
    prompt.push_str(&format!("File: {file}\n\n"));
    prompt.push_str("Recent file history:\n");

    for entry in history {
        prompt.push_str(&format!(
            "- Commit {} at {}\n  Subject: {}\n",
            entry.commit.short_hash, entry.commit.timestamp, entry.commit.subject
        ));

        if !entry.commit.body.trim().is_empty() {
            prompt.push_str(&format!(
                "  Message details: {}\n",
                compact_text(&entry.commit.body, 240)
            ));
        }

        if let Some(context) = &entry.context {
            if !context.analysis.is_empty() {
                prompt.push_str(&format!(
                    "  Detected intent: {}\n",
                    context.analysis.intent.summary()
                ));
                if let Some(signals) = context.analysis.intent.signals_summary(5) {
                    prompt.push_str(&format!(
                        "  Intent signals: {}\n",
                        compact_text(&signals, 240)
                    ));
                }
                if let Some(summary) = context.analysis.diff.summary() {
                    prompt.push_str(&format!("  Diff summary: {summary}\n"));
                }
                if let Some(summary) = context.analysis.diff.semantic_summary() {
                    prompt.push_str(&format!("  Semantic diff: {summary}\n"));
                }
                if let Some(summary) = context.analysis.impact.summary() {
                    prompt.push_str(&format!("  Impact summary: {summary}\n"));
                }
                if let Some(files) = context.analysis.diff.top_files_summary(5) {
                    prompt.push_str(&format!(
                        "  Most affected files: {}\n",
                        compact_text(&files, 240)
                    ));
                }
                if let Some(symbols) = context.analysis.diff.changed_symbols_summary(6) {
                    prompt.push_str(&format!(
                        "  Symbols touched: {}\n",
                        compact_text(&symbols, 240)
                    ));
                }
                if let Some(imports) = context.analysis.diff.import_summary(4) {
                    prompt.push_str(&format!(
                        "  Import changes: {}\n",
                        compact_text(&imports, 240)
                    ));
                }
                if let Some(dependents) = context.analysis.impact.top_direct_summary(4) {
                    prompt.push_str(&format!(
                        "  Direct dependents: {}\n",
                        compact_text(&dependents, 240)
                    ));
                }
                if let Some(cycles) = context.analysis.impact.circular_summary(2) {
                    prompt.push_str(&format!(
                        "  Circular dependencies: {}\n",
                        compact_text(&cycles, 240)
                    ));
                }
            }
            if !context.files.is_empty() {
                prompt.push_str(&format!(
                    "  Files changed: {}\n",
                    compact_join(&context.files, 8)
                ));
            }
            if !context.commands.is_empty() {
                prompt.push_str(&format!(
                    "  Developer commands: {}\n",
                    compact_join(&context.commands, 6)
                ));
            }
            if !context.environment.is_empty() {
                prompt.push_str(&format!(
                    "  Environment: {}\n",
                    compact_text(&context.environment.to_prompt_string(), 240)
                ));
            }
            if let Some(summary) = context.ide.summary() {
                prompt.push_str(&format!("  IDE context: {}\n", compact_text(&summary, 240)));
            }
            if let Some(summary) = context.review.summary() {
                prompt.push_str(&format!(
                    "  Review context: {}\n",
                    compact_text(&summary, 240)
                ));
            }
            if let Some(summary) = context.behavior.summary() {
                prompt.push_str(&format!(
                    "  Developer behavior: {}\n",
                    compact_text(&summary, 240)
                ));
            }
        }
    }

    if !predicted_files.is_empty() {
        prompt.push_str(&format!(
            "\nRelated files likely to matter next: {}\n",
            compact_join(predicted_files, 5)
        ));
    }

    prompt.push_str(
        "\nWrite a concise explanation in 1-3 short paragraphs. Mention the recent direction of the file, what the latest change appears to accomplish, and how the earlier commits set it up.\n",
    );
    prompt
}

fn heuristic_explanation(
    file: &str,
    history: &[HistoryEntry],
    predicted_files: &[String],
) -> String {
    let latest = &history[0];
    let earlier_subjects = history
        .iter()
        .skip(1)
        .take(3)
        .map(|entry| entry.commit.subject.as_str())
        .collect::<Vec<_>>();

    let command_hint = latest.context.as_ref().and_then(|context| {
        if context.commands.is_empty() {
            None
        } else {
            Some(compact_join(&context.commands, 3))
        }
    });

    let mut explanation = format!(
        "{} appears to have evolved through {} recent commit{}.",
        file,
        history.len(),
        if history.len() == 1 { "" } else { "s" }
    );
    explanation.push_str(&format!(
        " The latest recorded change is `{}` ({})",
        latest.commit.subject, latest.commit.short_hash
    ));

    if !earlier_subjects.is_empty() {
        explanation.push_str(&format!(
            ", building on earlier work such as {}.",
            earlier_subjects
                .iter()
                .map(|subject| format!("`{subject}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    } else {
        explanation.push('.');
    }

    if let Some(context) = &latest.context {
        if context.analysis.intent.category != ChangeCategory::Unknown {
            explanation.push_str(&format!(
                " The captured intent for the latest commit looks like a {} change with {} urgency and {} risk.",
                context.analysis.intent.category, context.analysis.intent.urgency, context.analysis.intent.risk
            ));
        }
        if let Some(signals) = context.analysis.intent.signals_summary(3) {
            explanation.push_str(&format!(" Intent clues included {}.", signals));
        }
        if let Some(summary) = context.analysis.diff.summary() {
            explanation.push_str(&format!(" The change footprint was {}.", summary));
        }
        if let Some(summary) = context.analysis.diff.semantic_summary() {
            explanation.push_str(&format!(" Semantically, it looks like {}.", summary));
        }
        if let Some(summary) = context.analysis.impact.summary() {
            explanation.push_str(&format!(" The broader impact looks like {}.", summary));
        }
        if let Some(symbols) = context.analysis.diff.changed_symbols_summary(3) {
            explanation.push_str(&format!(" The main symbols touched were {}.", symbols));
        }
        if let Some(imports) = context.analysis.diff.import_summary(2) {
            explanation.push_str(&format!(
                " Dependency-related changes included {}.",
                imports
            ));
        }
        if !context.files.is_empty() {
            explanation.push_str(&format!(
                " The commit also touched {}.",
                compact_join(&context.files, 5)
            ));
        }
        if let Some(review) = context.review.summary() {
            explanation.push_str(&format!(" Review context suggests {}.", review));
        }
        if let Some(behavior) = context.behavior.summary() {
            explanation.push_str(&format!(" Developer history shows {}.", behavior));
        }
    }

    if let Some(command_hint) = command_hint {
        explanation.push_str(&format!(
            " Captured developer activity around that commit included {}.",
            command_hint
        ));
    }

    if !predicted_files.is_empty() {
        explanation.push_str(&format!(
            " Related files likely worth checking next include {}.",
            compact_join(predicted_files, 3)
        ));
    }

    explanation
}

fn history_fingerprint(history: &[HistoryEntry]) -> String {
    let mut hasher = DefaultHasher::new();

    for entry in history {
        entry.commit.hash.hash(&mut hasher);
        entry.commit.timestamp.hash(&mut hasher);
        entry.commit.subject.hash(&mut hasher);
        entry.commit.body.hash(&mut hasher);

        if let Some(context) = &entry.context {
            context.commit.hash(&mut hasher);
            context.timestamp.hash(&mut hasher);
            context.commands.hash(&mut hasher);
            context.environment.hash(&mut hasher);
            context.ide.hash(&mut hasher);
            context.review.hash(&mut hasher);
            context.behavior.hash(&mut hasher);
            context.files.hash(&mut hasher);
            context.analysis.hash(&mut hasher);
        }
    }

    format!("{:016x}", hasher.finish())
}

fn try_cache_lookup(
    file: &str,
    latest_commit_hash: &str,
    history_hash: &str,
    has_api_key: bool,
    ai_model: &str,
) -> Option<CachedExplanation> {
    let cache_id =
        crate::storage::cache_manager::cache_key(file, latest_commit_hash, history_hash, ai_model);
    let record = crate::storage::cache_manager::get_explanation(&cache_id)?;
    let cache = CachedExplanation {
        source: record.source,
        ai_model: record.ai_model,
        last_commit_hash: record.commit_hash,
        commit_history_hash: record.commit_history_hash,
        explanation: record.explanation,
    };

    let matches_history =
        cache.last_commit_hash == latest_commit_hash && cache.commit_history_hash == history_hash;

    if !matches_history {
        return None;
    }

    if cache.source == "heuristic" && has_api_key {
        return None;
    }

    if cache.source == "ai" && !cache.ai_model.is_empty() && cache.ai_model != ai_model {
        return None;
    }

    Some(cache)
}

fn save_cache(
    file: &str,
    latest_commit_hash: &str,
    history_hash: &str,
    explanation: &str,
    source: &str,
    ai_model: &str,
    predicted_files: &[String],
) {
    let id =
        crate::storage::cache_manager::cache_key(file, latest_commit_hash, history_hash, ai_model);
    let record = crate::storage::cache_manager::ExplanationCacheRecord {
        id,
        commit_hash: latest_commit_hash.to_string(),
        file_path: file.to_string(),
        explanation: explanation.to_string(),
        metadata: json!({
            "predictive_candidates": predicted_files,
        }),
        source: source.to_string(),
        ai_model: ai_model.to_string(),
        commit_history_hash: history_hash.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_accessed: String::new(),
        access_count: 0,
    };

    crate::storage::cache_manager::put_explanation(&record);
}

fn log_ai_event(
    file: &str,
    cache_hit: bool,
    api_response_time: Option<f64>,
    error: Option<&str>,
    source: &str,
) {
    let Ok(log_dir) = crate::storage::log_dir() else {
        return;
    };
    if fs::create_dir_all(&log_dir).is_err() {
        return;
    }

    let Ok(log_path) = crate::storage::ai_log_path() else {
        return;
    };

    let mut entry = format!(
        "{}\nfile: {}\nsource: {}\ncache_hit: {}\n",
        chrono::Utc::now().to_rfc3339(),
        file,
        source,
        cache_hit
    );

    if let Some(api_response_time) = api_response_time {
        entry.push_str(&format!("api_time: {:.3}s\n", api_response_time));
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

fn extract_text(response: &serde_json::Value) -> Option<&str> {
    response
        .get("candidates")
        .and_then(|value| value.get(0))
        .and_then(|value| value.get("content"))
        .and_then(|value| value.get("parts"))
        .and_then(|value| value.get(0))
        .and_then(|value| value.get("text"))
        .and_then(|value| value.as_str())
}

fn compact_join(values: &[String], limit: usize) -> String {
    values
        .iter()
        .take(limit)
        .map(|value| format!("`{}`", compact_text(value, 80)))
        .collect::<Vec<_>>()
        .join(", ")
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

fn print_related_files(predicted_files: &[String]) {
    if !predicted_files.is_empty() {
        println!(
            "\nLikely related files: {}",
            predicted_files
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}
