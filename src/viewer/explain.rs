use crate::ai::AiBackend;
use crate::analysis::ChangeCategory;
use crate::config::AiProvider;
use crate::history::HistoryEntry;
use serde::Deserialize;
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::time::Instant;

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
    let local_cache_key = format!("local:{local_model}");
    let has_api_key = !api_key.trim().is_empty();
    let offline_mode = config.privacy.offline_mode;

    let history = match crate::history::load_history_for_file(&normalized_file, history_limit) {
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

    let base_history_hash = crate::history::history_fingerprint(&history);
    let fallback = heuristic_explanation(&normalized_file, &history, &predicted_files);

    let prompt_budget = config.ai.prompt_char_budget.max(2_000);
    let (prompt, prompt_detail, selected_indices) =
        build_prompt_with_budget(&normalized_file, &history, &predicted_files, prompt_budget);
    let history_hash = prompt_history_hash(
        &base_history_hash,
        prompt_budget,
        prompt_detail,
        &selected_indices,
    );
    let latest_commit_hash = history[0].commit.hash.clone();

    let mut backends =
        crate::ai::model_selector::choose_backends(&config.ai, &prompt, has_api_key, offline_mode);

    if backends.contains(&AiBackend::Local)
        && !crate::ai::local_ollama::is_available(&config.ai.ollama_url, 2)
    {
        backends.retain(|backend| *backend != AiBackend::Local);
    }

    let ai_available = !backends.is_empty();
    let cache_keys: Vec<&str> = if ai_available {
        backends
            .iter()
            .copied()
            .map(|backend| backend_cache_key(backend, cloud_model, local_cache_key.as_str()))
            .collect()
    } else {
        vec![cloud_model, local_cache_key.as_str()]
    };

    for cache_key in cache_keys {
        if let Some(cache) = try_cache_lookup(
            &normalized_file,
            &latest_commit_hash,
            &history_hash,
            ai_available,
            cache_key,
        ) {
            let heading = if cache.source == "ai" {
                "AI Explanation"
            } else {
                "Explanation"
            };
            println!("Using cached {} for {}\n", cache.source, normalized_file);
            println!("{heading}:\n{}", cache.explanation);
            print_related_files(&predicted_files);
            log_ai_event(
                &normalized_file,
                true,
                None,
                None,
                &cache.source,
                Some(&cache.ai_model),
            );
            return;
        }
    }

    if !ai_available {
        println!("Explanation:\n{}", fallback);
        save_cache(
            &normalized_file,
            &latest_commit_hash,
            &history_hash,
            &fallback,
            "heuristic",
            cloud_model,
            &predicted_files,
        );
        print_related_files(&predicted_files);
        let reason = match config.ai.provider {
            AiProvider::Cloud => {
                if offline_mode {
                    "offline_mode_enabled"
                } else {
                    "missing_api_key"
                }
            }
            AiProvider::Local => "local_ai_unavailable",
            AiProvider::Hybrid => "no_ai_backend_available",
        };
        log_ai_event(
            &normalized_file,
            false,
            None,
            Some(reason),
            "heuristic",
            Some(cloud_model),
        );
        return;
    }

    println!(
        "Generating AI explanation for {} using {}...\n",
        normalized_file,
        backends[0].as_str()
    );

    let mut last_error: Option<String> = None;
    let mut last_elapsed: Option<f64> = None;
    let mut last_model_key: Option<String> = None;

    for backend in backends {
        let model_key =
            backend_cache_key(backend, cloud_model, local_cache_key.as_str()).to_string();
        let started = Instant::now();
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
        let elapsed = started.elapsed().as_secs_f64();

        match result {
            Ok(text) => {
                println!("AI Explanation:\n{}", text);
                print_related_files(&predicted_files);
                save_cache(
                    &normalized_file,
                    &latest_commit_hash,
                    &history_hash,
                    &text,
                    "ai",
                    &model_key,
                    &predicted_files,
                );
                log_ai_event(
                    &normalized_file,
                    false,
                    Some(elapsed),
                    None,
                    "ai",
                    Some(&model_key),
                );
                return;
            }
            Err(error) => {
                last_error = Some(error.to_string());
                last_elapsed = Some(elapsed);
                last_model_key = Some(model_key);
            }
        }
    }

    println!("Explanation:\n{}", fallback);
    save_cache(
        &normalized_file,
        &latest_commit_hash,
        &history_hash,
        &fallback,
        "heuristic",
        cloud_model,
        &predicted_files,
    );
    print_related_files(&predicted_files);
    log_ai_event(
        &normalized_file,
        false,
        last_elapsed,
        last_error.as_deref(),
        "heuristic",
        last_model_key.as_deref().or(Some(cloud_model)),
    );
}

fn build_prompt_with_budget(
    file: &str,
    history: &[HistoryEntry],
    predicted_files: &[String],
    budget_chars: usize,
) -> (String, crate::ai::reasoning_chain::PromptDetail, Vec<usize>) {
    let ranked = crate::ai::context_optimizer::rank_history(file, history);
    let max_commits = history.len().min(25).max(1);

    let mut selected = Vec::with_capacity(max_commits);
    selected.push(0);
    for entry in ranked
        .iter()
        .filter(|entry| entry.index != 0)
        .take(max_commits - 1)
    {
        selected.push(entry.index);
    }
    selected.sort_unstable();

    let mut detail = crate::ai::reasoning_chain::PromptDetail::Full;

    loop {
        let selected_history = selected
            .iter()
            .filter_map(|index| history.get(*index).cloned())
            .collect::<Vec<_>>();

        let prompt = crate::ai::reasoning_chain::build_explain_prompt(
            file,
            &selected_history,
            predicted_files,
            detail,
        );

        if prompt.chars().count() <= budget_chars {
            return (prompt, detail, selected);
        }

        detail = match detail {
            crate::ai::reasoning_chain::PromptDetail::Full => {
                crate::ai::reasoning_chain::PromptDetail::Compact
            }
            crate::ai::reasoning_chain::PromptDetail::Compact => {
                crate::ai::reasoning_chain::PromptDetail::Minimal
            }
            crate::ai::reasoning_chain::PromptDetail::Minimal => {
                if selected.len() <= 1 {
                    return (truncate_to_budget(&prompt, budget_chars), detail, selected);
                }

                if let Some(drop_index) =
                    crate::ai::context_optimizer::pick_drop_candidate(&selected, &ranked)
                {
                    selected.retain(|index| *index != drop_index);
                } else {
                    selected.pop();
                }

                crate::ai::reasoning_chain::PromptDetail::Minimal
            }
        };
    }
}

fn prompt_history_hash(
    base_history_hash: &str,
    budget_chars: usize,
    detail: crate::ai::reasoning_chain::PromptDetail,
    selected_indices: &[usize],
) -> String {
    let mut hasher = DefaultHasher::new();

    2u8.hash(&mut hasher); // prompt format version
    base_history_hash.hash(&mut hasher);
    budget_chars.hash(&mut hasher);

    let detail_marker: u8 = match detail {
        crate::ai::reasoning_chain::PromptDetail::Full => 1,
        crate::ai::reasoning_chain::PromptDetail::Compact => 2,
        crate::ai::reasoning_chain::PromptDetail::Minimal => 3,
    };
    detail_marker.hash(&mut hasher);
    selected_indices.hash(&mut hasher);

    format!("{:016x}", hasher.finish())
}

fn truncate_to_budget(input: &str, budget_chars: usize) -> String {
    let len = input.chars().count();
    if len <= budget_chars {
        return input.to_string();
    }

    if budget_chars <= 3 {
        return ".".repeat(budget_chars);
    }

    let prefix = input.chars().take(budget_chars - 3).collect::<String>();
    format!("{prefix}...")
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

fn backend_cache_key<'a>(
    backend: AiBackend,
    cloud_model: &'a str,
    local_cache_key: &'a str,
) -> &'a str {
    match backend {
        AiBackend::Cloud => cloud_model,
        AiBackend::Local => local_cache_key,
    }
}

fn try_cache_lookup(
    file: &str,
    latest_commit_hash: &str,
    history_hash: &str,
    ai_available: bool,
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

    if cache.source == "heuristic" && ai_available {
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
    ai_model: Option<&str>,
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

    if let Some(ai_model) = ai_model {
        entry.push_str(&format!("ai_model: {}\n", ai_model));
    }

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
