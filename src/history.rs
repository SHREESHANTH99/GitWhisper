use crate::error::AppResult;
use crate::git::FileCommit;
use crate::storage::context::CommitContext;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub commit: FileCommit,
    pub context: Option<CommitContext>,
}

pub fn load_history_for_file(file: &str, limit: usize) -> AppResult<Vec<HistoryEntry>> {
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

pub fn history_fingerprint(history: &[HistoryEntry]) -> String {
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
