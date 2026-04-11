use crate::history::HistoryEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptDetail {
    Full,
    Compact,
    Minimal,
}

pub fn build_explain_prompt(
    file: &str,
    history: &[HistoryEntry],
    predicted_files: &[String],
    detail: PromptDetail,
) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are helping a developer understand why a file changed over time.\n");
    prompt.push_str("Focus on intent, the problem being solved, and how the file evolved.\n");
    prompt.push_str("Only make claims supported by the evidence. If something is uncertain, say it likely happened.\n");
    prompt.push_str("Do your reasoning internally, but only output the final explanation.\n\n");

    prompt.push_str(&format!("File: {file}\n\n"));
    prompt.push_str("Evidence (recent commits considered):\n");

    for (idx, entry) in history.iter().enumerate() {
        prompt.push_str(&format!(
            "- Commit {} at {}\n  Subject: {}\n",
            entry.commit.short_hash, entry.commit.timestamp, entry.commit.subject
        ));

        if detail == PromptDetail::Full && idx == 0 && !entry.commit.body.trim().is_empty() {
            prompt.push_str(&format!(
                "  Message details: {}\n",
                compact_text(&entry.commit.body, 240)
            ));
        }

        let Some(context) = &entry.context else {
            continue;
        };

        if detail == PromptDetail::Minimal {
            continue;
        }

        if !context.analysis.is_empty() {
            prompt.push_str(&format!(
                "  Detected intent: {}\n",
                context.analysis.intent.summary()
            ));

            if detail == PromptDetail::Full && idx == 0 {
                if let Some(signals) = context.analysis.intent.signals_summary(5) {
                    prompt.push_str(&format!(
                        "  Intent signals: {}\n",
                        compact_text(&signals, 240)
                    ));
                }
            }

            if let Some(summary) = context.analysis.diff.summary() {
                prompt.push_str(&format!("  Diff summary: {summary}\n"));
            }

            if detail == PromptDetail::Full && idx == 0 {
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
            } else if detail == PromptDetail::Compact && idx == 0 {
                if let Some(summary) = context.analysis.impact.summary() {
                    prompt.push_str(&format!("  Impact summary: {summary}\n"));
                }
            }
        }

        if detail == PromptDetail::Full && idx == 0 {
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

    if detail != PromptDetail::Minimal && !predicted_files.is_empty() {
        prompt.push_str(&format!(
            "\nRelated files likely to matter next: {}\n",
            compact_join(predicted_files, 5)
        ));
    }

    prompt.push_str(
        "\nTask: Write a concise explanation in 1-3 short paragraphs. Mention the recent direction of the file, what the latest change appears to accomplish, and how the earlier commits set it up.\n",
    );
    prompt
}

pub fn build_file_evolution_prompt(file: &str, history: &[HistoryEntry], detail: PromptDetail) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are helping a developer understand how a file evolved over time.\n");
    prompt.push_str("Only make claims supported by the evidence. If uncertain, use probabilistic language.\n");
    prompt.push_str("Do your reasoning internally, but only output the final summary.\n\n");
    prompt.push_str(&format!("File: {file}\n\n"));

    prompt.push_str("Evidence (commits considered):\n");
    for (idx, entry) in history.iter().enumerate() {
        prompt.push_str(&format!(
            "- {} at {}: {}\n",
            entry.commit.short_hash, entry.commit.timestamp, entry.commit.subject
        ));

        if detail == PromptDetail::Full && idx == 0 && !entry.commit.body.trim().is_empty() {
            prompt.push_str(&format!(
                "  Message details: {}\n",
                compact_text(&entry.commit.body, 240)
            ));
        }

        if detail == PromptDetail::Minimal {
            continue;
        }

        if let Some(context) = &entry.context {
            if !context.analysis.is_empty() {
                prompt.push_str(&format!(
                    "  Intent: {}\n",
                    compact_text(&context.analysis.intent.summary(), 240)
                ));
                if let Some(summary) = context.analysis.diff.summary() {
                    prompt.push_str(&format!("  Diff: {summary}\n"));
                }
            }
        }
    }

    prompt.push_str(
        "\nTask: Write a short file evolution narrative. Include:\n- A 3-6 bullet milestone timeline (newest to oldest is fine)\n- A 1-2 paragraph story describing phases (creation, expansion, hardening/refactor), and any notable shifts in intent or impact.\n",
    );

    prompt
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

