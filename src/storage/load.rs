use crate::storage::context::CommitContext;
use std::fs;
use std::path::PathBuf;

pub fn load_context(commit_prefix: &str) -> Result<CommitContext, String> {
    let file_path = context_files()?
        .into_iter()
        .find(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(|stem| stem.starts_with(commit_prefix))
                .unwrap_or(false)
        })
        .ok_or_else(|| format!("No captured context found for commit prefix `{commit_prefix}`."))?;

    let raw = fs::read_to_string(&file_path)
        .map_err(|error| format!("Failed to read {}: {}", file_path.display(), error))?;
    serde_json::from_str::<CommitContext>(&raw)
        .map_err(|error| format!("Failed to parse {}: {}", file_path.display(), error))
}

pub fn load_all_contexts() -> Result<Vec<CommitContext>, String> {
    let mut contexts = Vec::new();
    for file_path in context_files()? {
        let Ok(raw) = fs::read_to_string(&file_path) else {
            continue;
        };
        let Ok(context) = serde_json::from_str::<CommitContext>(&raw) else {
            continue;
        };
        contexts.push(context);
    }

    contexts.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
    Ok(contexts)
}

pub fn latest_context() -> Result<CommitContext, String> {
    load_all_contexts()?
        .into_iter()
        .next()
        .ok_or_else(|| "No captured commit context found yet.".to_string())
}

fn context_files() -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let primary_dir = crate::storage::app_dir()?;
    collect_context_files(&primary_dir, &mut files)?;

    let legacy_dir = crate::storage::legacy_app_dir()?;
    if legacy_dir != primary_dir {
        collect_context_files(&legacy_dir, &mut files)?;
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn collect_context_files(context_dir: &PathBuf, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = match fs::read_dir(context_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "Failed to read context directory {}: {}",
                context_dir.display(),
                error
            ))
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            files.push(path);
        }
    }

    Ok(())
}
