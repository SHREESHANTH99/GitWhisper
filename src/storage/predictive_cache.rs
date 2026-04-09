use crate::storage::context::CommitContext;
use std::collections::HashMap;

pub fn predict_related_files(
    current_file: &str,
    latest_context: Option<&CommitContext>,
    historical_contexts: &[CommitContext],
    limit: usize,
) -> Vec<String> {
    let mut scores: HashMap<String, i32> = HashMap::new();

    if let Some(context) = latest_context {
        for file in &context.analysis.impact.direct_dependents {
            if file != current_file {
                *scores.entry(file.clone()).or_insert(0) += 12;
            }
        }

        for file in &context.analysis.impact.transitive_dependents {
            if file != current_file {
                *scores.entry(file.clone()).or_insert(0) += 6;
            }
        }
    }

    for context in historical_contexts {
        if !context.files.iter().any(|file| file == current_file) {
            continue;
        }

        for file in &context.files {
            if file != current_file {
                *scores.entry(file.clone()).or_insert(0) += 4;
            }
        }
    }

    let mut scored = scores.into_iter().collect::<Vec<_>>();
    scored.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    scored
        .into_iter()
        .take(limit)
        .map(|(file, _)| file)
        .collect()
}
