use crate::history::HistoryEntry;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct RankedHistoryIndex {
    pub index: usize,
    pub score: f32,
}

pub fn rank_history(file: &str, history: &[HistoryEntry]) -> Vec<RankedHistoryIndex> {
    if history.is_empty() {
        return Vec::new();
    }

    let mut query_tokens = tokenize(file);
    query_tokens.extend(tokenize(&history[0].commit.subject));
    let query_tokens = query_tokens;

    let mut ranked = Vec::with_capacity(history.len());
    for (index, entry) in history.iter().enumerate() {
        let score = score_entry(index, entry, &query_tokens);
        ranked.push(RankedHistoryIndex { index, score });
    }

    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.index.cmp(&b.index))
    });
    ranked
}

pub fn pick_drop_candidate(selected: &[usize], ranked: &[RankedHistoryIndex]) -> Option<usize> {
    let mut worst: Option<(usize, f32)> = None;
    for index in selected.iter().copied() {
        if index == 0 {
            continue;
        }

        let score = ranked
            .iter()
            .find(|entry| entry.index == index)
            .map(|entry| entry.score)
            .unwrap_or(0.0);

        worst = match worst {
            None => Some((index, score)),
            Some((_, worst_score)) if score < worst_score => Some((index, score)),
            Some(existing) => Some(existing),
        };
    }

    worst.map(|(index, _)| index)
}

fn score_entry(index: usize, entry: &HistoryEntry, query_tokens: &[String]) -> f32 {
    let mut score = 0.0;

    // Always keep the newest commit.
    if index == 0 {
        score += 100.0;
    }

    let recency_bonus = 1.0 / (1.0 + index as f32);
    score += recency_bonus * 2.0;

    let entry_tokens = entry_tokens(entry);
    let overlap = query_tokens
        .iter()
        .filter(|token| entry_tokens.contains(token.as_str()))
        .count() as f32;
    score += overlap * 3.0;

    if let Some(context) = &entry.context {
        if !context.analysis.is_empty() {
            score += 0.5;
        }

        let churn = context.analysis.diff.lines_added + context.analysis.diff.lines_removed;
        if churn >= 200 {
            score += 2.0;
        } else if churn >= 50 {
            score += 1.0;
        }
    }

    score
}

fn entry_tokens(entry: &HistoryEntry) -> HashSet<String> {
    let mut tokens = tokenize(&entry.commit.subject);
    tokens.extend(tokenize(&entry.commit.body));

    if let Some(context) = &entry.context {
        if !context.analysis.intent.conventional_type.trim().is_empty() {
            tokens.extend(tokenize(&context.analysis.intent.conventional_type));
        }
        if !context.analysis.intent.conventional_scope.trim().is_empty() {
            tokens.extend(tokenize(&context.analysis.intent.conventional_scope));
        }

        if let Some(summary) = context.analysis.diff.semantic_summary() {
            tokens.extend(tokenize(&summary));
        }
        if let Some(summary) = context.analysis.impact.summary() {
            tokens.extend(tokenize(&summary));
        }
    }

    tokens.into_iter().collect()
}

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            push_token(&mut tokens, &mut current);
        }
    }

    if !current.is_empty() {
        push_token(&mut tokens, &mut current);
    }

    tokens
}

fn push_token(tokens: &mut Vec<String>, current: &mut String) {
    let token = current.trim().to_string();
    current.clear();

    if token.len() >= 2 {
        tokens.push(token);
    }
}

