use crate::analysis::{ChangeCategory, ChangeScope, RiskLevel};
use crate::error::AppResult;
use crate::storage::context::CommitContext;
use std::fs;
use std::path::PathBuf;

pub fn generate_adrs(output_dir: &str) {
    match generate_adrs_inner(output_dir) {
        Ok(path) => println!("Generated ADRs at {}", path.display()),
        Err(error) => eprintln!("{error}"),
    }
}

fn generate_adrs_inner(output_dir: &str) -> AppResult<PathBuf> {
    let contexts = crate::storage::load::load_all_contexts()?;
    let output = PathBuf::from(output_dir);
    fs::create_dir_all(&output)?;

    let mut records = contexts
        .iter()
        .filter(|context| is_decision_worthy(context))
        .cloned()
        .collect::<Vec<_>>();

    records.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));

    let mut index = String::new();
    index.push_str("# Architectural Decision Records\n\n");

    for (idx, context) in records.iter().enumerate() {
        let adr_number = idx + 1;
        let file_name = format!("ADR-{:03}-{}.md", adr_number, slugify(&subject_for_context(context)));
        fs::write(output.join(&file_name), render_adr(adr_number, context))?;
        index.push_str(&format!(
            "- [ADR-{:03}: {}]({})\n",
            adr_number,
            subject_for_context(context),
            file_name
        ));
    }

    if records.is_empty() {
        index.push_str("No decision-worthy commits were detected from the captured context.\n");
    }

    fs::write(output.join("README.md"), index)?;
    Ok(output)
}

fn render_adr(number: usize, context: &CommitContext) -> String {
    let subject = subject_for_context(context);
    let author = if !context.behavior.author.trim().is_empty() {
        context.behavior.author.clone()
    } else {
        crate::git::commit_author_name(&context.commit).unwrap_or_else(|_| "unknown".to_string())
    };
    let mut text = String::new();
    text.push_str(&format!("# ADR-{:03}: {}\n\n", number, subject));
    text.push_str(&format!("## Status\nAccepted (Commit: {})\n\n", context.commit));
    text.push_str(&format!("## Date\n{}\n\n", context.timestamp));
    text.push_str(&format!("## Author\n{}\n\n", author));
    text.push_str("## Context\n");
    if let Some(summary) = context.analysis.diff.summary() {
        text.push_str(&format!("{summary}\n\n"));
    } else {
        text.push_str("Captured commit context suggests a notable architectural or product decision.\n\n");
    }
    text.push_str("## Decision\n");
    text.push_str(&format!(
        "{}\n\n",
        context
            .analysis
            .intent
            .signals_summary(5)
            .unwrap_or_else(|| subject.clone())
    ));
    text.push_str("## Consequences\n");
    if let Some(summary) = context.analysis.impact.summary() {
        text.push_str(&format!("- {}\n", summary));
    } else {
        text.push_str("- Broader impact was not fully captured.\n");
    }
    text.push_str(&format!(
        "- Risk level: {}\n- Scope: {}\n",
        context.analysis.intent.risk,
        context.analysis.intent.scope
    ));
    text.push_str("\n## Related Files\n");
    if context.files.is_empty() {
        text.push_str("- No file list captured\n");
    } else {
        for file in &context.files {
            text.push_str(&format!("- {}\n", file));
        }
    }
    text
}

fn is_decision_worthy(context: &CommitContext) -> bool {
    let subject = subject_for_context(context).to_ascii_lowercase();
    let category = &context.analysis.intent.category;
    let risk = &context.analysis.intent.risk;
    let scope = &context.analysis.intent.scope;

    matches!(category, ChangeCategory::Feature | ChangeCategory::Refactor | ChangeCategory::Performance)
        || matches!(risk, RiskLevel::High | RiskLevel::Critical)
        || matches!(scope, ChangeScope::Broad)
        || context.analysis.intent.breaking_change
        || subject.contains("migrate")
        || subject.contains("architecture")
        || subject.contains("switch")
        || subject.contains("replace")
        || subject.contains("adopt")
        || subject.contains("introduce")
        || subject.contains("implement")
}

fn subject_for_context(context: &CommitContext) -> String {
    crate::git::commit_subject(&context.commit).unwrap_or_else(|_| "Untitled decision".to_string())
}

fn slugify(input: &str) -> String {
    let mut output = String::new();
    let mut last_dash = false;

    for ch in input.chars() {
        let normalized = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '-'
        };

        if normalized == '-' {
            if !last_dash {
                output.push('-');
                last_dash = true;
            }
        } else {
            output.push(normalized);
            last_dash = false;
        }
    }

    output.trim_matches('-').to_string()
}

