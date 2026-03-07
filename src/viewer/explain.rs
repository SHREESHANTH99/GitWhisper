use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Instant;

const CONTEXT_DIR: &str = ".git/commitlens";
const CACHE_DIR: &str = ".git/commitlens/cache";
const LOG_DIR: &str = ".git/commitlens/logs";
const LOG_FILE: &str = ".git/commitlens/logs/ai.log";
const AI_MODEL: &str = "gemini-1.5-flash";
const HISTORY_LIMIT: usize = 10;

#[derive(Deserialize, Clone)]
struct CommitContext {
    commit: String,
    timestamp: String,
    #[serde(default)]
    commands: Vec<String>,
    #[serde(default)]
    environment: String,
    #[serde(default)]
    os: String,
    #[serde(default)]
    node: Option<String>,
}

#[derive(Deserialize)]
struct CachedExplanation {
    last_commit_hash: String,
    commit_history_hash: String,
    explanation: String,
}

#[derive(Serialize)]
struct ExplanationCacheEntry<'a> {
    file: &'a str,
    last_commit_hash: &'a str,
    generated_at: String,
    ai_model: &'a str,
    explanation: &'a str,
    commit_history_hash: &'a str,
}

fn cache_path_for_file(file: &str) -> PathBuf {
    // Use a stable hash of the full file path to avoid collisions and
    // platform-specific filename issues.
    let mut hasher = DefaultHasher::new();
    file.hash(&mut hasher);
    let hash = hasher.finish();

    // Use the file's basename (if available) as a human-readable prefix, but
    // sanitize it to ensure it is safe as a filename component.
    let basename = Path::new(file)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file");

    let safe_prefix: String = basename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let filename = format!("{}-{:016x}.json", safe_prefix, hash);
    Path::new(CACHE_DIR).join(filename)
}

fn log_ai_event(file: &str, cache_hit: bool, api_response_time: Option<f64>, error: Option<&str>) {
    if fs::create_dir_all(LOG_DIR).is_err() {
        return;
    }

    let mut log_entry = format!(
        "{}\nfile: {}\ncache_hit: {}\n",
        chrono::Utc::now().to_rfc3339(),
        file,
        cache_hit
    );

    if let Some(time) = api_response_time {
        log_entry.push_str(&format!("api_time: {:.3}s\n", time));
    }

    if let Some(err) = error {
        log_entry.push_str(&format!("error: {}\n", err));
    }

    log_entry.push('\n');

    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)
        .and_then(|mut file_handle| {
            std::io::Write::write_all(&mut file_handle, log_entry.as_bytes())
        });
}

fn load_history_for_file(file: &str) -> Vec<CommitContext> {
    let mut history = Vec::new();

    if let Ok(entries) = fs::read_dir(CONTEXT_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            if path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                == Some("cache")
            {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&path) {
                if !content.contains(file) {
                    continue;
                }

                if let Ok(ctx) = serde_json::from_str::<CommitContext>(&content) {
                    history.push(ctx);
                }
            }
        }
    }

    history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    history.truncate(HISTORY_LIMIT);
    history
}

fn history_fingerprint(history: &[CommitContext]) -> String {
    let mut hasher = DefaultHasher::new();

    for ctx in history {
        ctx.commit.hash(&mut hasher);
        ctx.timestamp.hash(&mut hasher);
        ctx.commands.hash(&mut hasher);
        ctx.environment.hash(&mut hasher);
        ctx.os.hash(&mut hasher);
        ctx.node.hash(&mut hasher);
    }

    format!("{:016x}", hasher.finish())
}

fn build_prompt(file: &str, history: &[CommitContext]) -> String {
    let mut prompt = format!(
        "Summarize the evolution of file {file} based on its most relevant recent commit context.\n"
    );

    for ctx in history {
        let env = if !ctx.environment.is_empty() {
            ctx.environment.clone()
        } else {
            format!(
                "OS: {} | Node: {}",
                ctx.os,
                ctx.node.clone().unwrap_or_else(|| "unknown".to_string())
            )
        };

        prompt.push_str(&format!(
            "- Commit {} at {}\n  Commands: {:?}\n  Environment: {}\n",
            ctx.commit, ctx.timestamp, ctx.commands, env
        ));
    }

    prompt.push_str("\nProvide a concise explanation of why these changes likely happened.");
    prompt
}

fn save_cache(file: &str, last_commit_hash: &str, commit_history_hash: &str, explanation: &str) {
    if fs::create_dir_all(CACHE_DIR).is_err() {
        return;
    }

    let cache_entry = ExplanationCacheEntry {
        file,
        last_commit_hash,
        generated_at: chrono::Utc::now().to_rfc3339(),
        ai_model: AI_MODEL,
        explanation,
        commit_history_hash,
    };

    if let Ok(json) = serde_json::to_string_pretty(&cache_entry) {
        let _ = fs::write(cache_path_for_file(file), json);
    }
}

fn try_cache_lookup(
    file: &str,
    latest_commit_hash: &str,
    commit_history_hash: &str,
) -> Option<String> {
    let cache_path = cache_path_for_file(file);
    let content = fs::read_to_string(cache_path).ok()?;
    let cache: CachedExplanation = serde_json::from_str(&content).ok()?;

    if cache.last_commit_hash == latest_commit_hash
        && cache.commit_history_hash == commit_history_hash
    {
        Some(cache.explanation)
    } else {
        None
    }
}

/// Explain changes to a file using Gemini API with caching.
pub fn explain_file(file: &str, api_key: &str) {
    let history = load_history_for_file(file);

    if history.is_empty() {
        println!("No commit context found for {}", file);
        return;
    }

    let latest_commit_hash = history[0].commit.clone();
    let commit_history_hash = history_fingerprint(&history);

    if let Some(cached) = try_cache_lookup(file, &latest_commit_hash, &commit_history_hash) {
        println!("✔ Using cached AI explanation");
        println!("File: {}", file);
        println!("\nAI Explanation:\n{}", cached);
        log_ai_event(file, true, None, None);
        return;
    }

    println!("⚡ Generating AI explanation...");
    let prompt = build_prompt(file, &history);

    if api_key.is_empty() {
        let fallback = format!(
            "AI unavailable (missing API key). Showing commit history summary for {}. Latest commits: {}",
            file,
            history
                .iter()
                .map(|ctx| ctx.commit.clone())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!("{}", fallback);
        save_cache(file, &latest_commit_hash, &commit_history_hash, &fallback);
        println!("✔ Explanation cached");
        log_ai_event(file, false, None, Some("missing_api_key"));
        return;
    }

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        AI_MODEL, api_key
    );

    let client = Client::new();
    let started = Instant::now();
    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }]
        }))
        .send();
    let elapsed = started.elapsed().as_secs_f64();

    match resp {
        Ok(r) => {
            let json_resp: serde_json::Value =
                r.json::<serde_json::Value>().unwrap_or_else(|_| json!({}));

            if let Some(text) = json_resp
                .get("candidates")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("content"))
                .and_then(|c| c.get("parts"))
                .and_then(|p| p.get(0))
                .and_then(|p| p.get("text"))
                .and_then(|t| t.as_str())
            {
                println!("AI Explanation:\n{}", text);
                save_cache(file, &latest_commit_hash, &commit_history_hash, text);
                println!("✔ Explanation cached");
                log_ai_event(file, false, Some(elapsed), None);
            } else {
                let message = json_resp
                    .get("error")
                    .and_then(|error| error.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown API error");

                let fallback = format!(
                    "AI unavailable. Showing commit history summary for {}. Latest commits: {}",
                    file,
                    history
                        .iter()
                        .map(|ctx| ctx.commit.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                println!("{}", fallback);
                save_cache(file, &latest_commit_hash, &commit_history_hash, &fallback);
                println!("✔ Explanation cached");
                log_ai_event(file, false, Some(elapsed), Some(message));
            }
        }
        Err(e) => {
            let fallback = format!(
                "AI unavailable. Showing commit history summary for {}. Latest commits: {}",
                file,
                history
                    .iter()
                    .map(|ctx| ctx.commit.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            println!("{}", fallback);
            save_cache(file, &latest_commit_hash, &commit_history_hash, &fallback);
            println!("✔ Explanation cached");
            log_ai_event(file, false, Some(elapsed), Some(&e.to_string()));
        }
    }
}
