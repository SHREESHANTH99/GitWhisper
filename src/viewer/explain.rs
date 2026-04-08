use crate::git::FileCommit;
use crate::storage::context::CommitContext;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const AI_MODEL: &str = "gemini-1.5-flash";
const HISTORY_LIMIT: usize = 10;

#[derive(Debug, Clone)]
struct HistoryEntry {
    commit: FileCommit,
    context: Option<CommitContext>,
}

#[derive(Debug, Deserialize)]
struct CachedExplanation {
    #[serde(default = "default_cache_source")]
    source: String,
    last_commit_hash: String,
    commit_history_hash: String,
    explanation: String,
}

#[derive(Debug, Serialize)]
struct ExplanationCacheEntry<'a> {
    file: &'a str,
    source: &'a str,
    last_commit_hash: &'a str,
    generated_at: String,
    ai_model: &'a str,
    explanation: &'a str,
    commit_history_hash: &'a str,
}

pub fn explain_file(file: &str, api_key: &str) {
    let normalized_file = match crate::git::normalize_repo_path(file) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    let history = match load_history_for_file(&normalized_file) {
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

    let latest_commit_hash = history[0].commit.short_hash.clone();
    let history_hash = history_fingerprint(&history);

    if let Some(cache) = try_cache_lookup(
        &normalized_file,
        &latest_commit_hash,
        &history_hash,
        !api_key.is_empty(),
    ) {
        let heading = if cache.source == "ai" {
            "AI Explanation"
        } else {
            "Explanation"
        };
        println!("Using cached {} for {}\n", cache.source, normalized_file);
        println!("{heading}:\n{}", cache.explanation);
        log_ai_event(&normalized_file, true, None, None, &cache.source);
        return;
    }

    let fallback = heuristic_explanation(&normalized_file, &history);
    if api_key.is_empty() {
        println!("Explanation:\n{}", fallback);
        save_cache(
            &normalized_file,
            &latest_commit_hash,
            &history_hash,
            &fallback,
            "heuristic",
        );
        log_ai_event(
            &normalized_file,
            false,
            None,
            Some("missing_api_key"),
            "heuristic",
        );
        return;
    }

    println!("Generating AI explanation for {}...\n", normalized_file);
    let prompt = build_prompt(&normalized_file, &history);
    let client = match Client::builder().timeout(Duration::from_secs(45)).build() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Failed to create HTTP client: {}", error);
            println!("Explanation:\n{}", fallback);
            return;
        }
    };

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        AI_MODEL, api_key
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
                save_cache(
                    &normalized_file,
                    &latest_commit_hash,
                    &history_hash,
                    text,
                    "ai",
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

fn load_history_for_file(file: &str) -> Result<Vec<HistoryEntry>, String> {
    let git_history = crate::git::file_history(file, HISTORY_LIMIT)?;
    let entries = git_history
        .into_iter()
        .map(|commit| {
            let context = crate::storage::load::load_context(&commit.short_hash).ok();
            HistoryEntry { commit, context }
        })
        .collect();

    Ok(entries)
}

fn build_prompt(file: &str, history: &[HistoryEntry]) -> String {
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
            if !context.environment.trim().is_empty() {
                prompt.push_str(&format!(
                    "  Environment: {}\n",
                    compact_text(&context.environment.replace('\n', " | "), 240)
                ));
            }
        }
    }

    prompt.push_str(
        "\nWrite a concise explanation in 1-3 short paragraphs. Mention the recent direction of the file, what the latest change appears to accomplish, and how the earlier commits set it up.\n",
    );
    prompt
}

fn heuristic_explanation(file: &str, history: &[HistoryEntry]) -> String {
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

    if let Some(command_hint) = command_hint {
        explanation.push_str(&format!(
            " Captured developer activity around that commit included {}.",
            command_hint
        ));
    }

    if let Some(context) = &latest.context {
        if !context.files.is_empty() {
            explanation.push_str(&format!(
                " The commit also touched {}.",
                compact_join(&context.files, 5)
            ));
        }
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
            context.files.hash(&mut hasher);
        }
    }

    format!("{:016x}", hasher.finish())
}

fn try_cache_lookup(
    file: &str,
    latest_commit_hash: &str,
    history_hash: &str,
    has_api_key: bool,
) -> Option<CachedExplanation> {
    let path = cache_path_for_file(file);
    let raw = fs::read_to_string(path).ok()?;
    let cache = serde_json::from_str::<CachedExplanation>(&raw).ok()?;

    let matches_history =
        cache.last_commit_hash == latest_commit_hash && cache.commit_history_hash == history_hash;

    if !matches_history {
        return None;
    }

    if cache.source == "heuristic" && has_api_key {
        None
    } else {
        Some(cache)
    }
}

fn save_cache(
    file: &str,
    latest_commit_hash: &str,
    history_hash: &str,
    explanation: &str,
    source: &str,
) {
    let Ok(cache_dir) = crate::storage::cache_dir() else {
        return;
    };

    if fs::create_dir_all(&cache_dir).is_err() {
        return;
    }

    let entry = ExplanationCacheEntry {
        file,
        source,
        last_commit_hash: latest_commit_hash,
        generated_at: chrono::Utc::now().to_rfc3339(),
        ai_model: AI_MODEL,
        explanation,
        commit_history_hash: history_hash,
    };

    let Ok(json) = serde_json::to_string_pretty(&entry) else {
        return;
    };

    let _ = fs::write(cache_path_for_file(file), json);
}

fn cache_path_for_file(file: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    file.hash(&mut hasher);
    let hash = hasher.finish();

    let basename = Path::new(file)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    let safe_prefix: String = basename
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect();

    let cache_dir =
        crate::storage::cache_dir().unwrap_or_else(|_| PathBuf::from(".git/gitwhisper/cache"));
    cache_dir.join(format!("{}-{:016x}.json", safe_prefix, hash))
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
    if collapsed.len() <= max_len {
        collapsed
    } else {
        format!("{}...", &collapsed[..max_len.saturating_sub(3)])
    }
}

fn default_cache_source() -> String {
    "ai".to_string()
}
