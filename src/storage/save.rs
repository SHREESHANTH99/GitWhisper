use crate::storage::context::CommitContext;
use std::fs;
use std::path::PathBuf;

pub fn save_context(context: &CommitContext) -> Result<PathBuf, String> {
    let app_dir = crate::storage::app_dir()?;
    fs::create_dir_all(&app_dir)
        .map_err(|error| format!("Failed to create {}: {}", app_dir.display(), error))?;

    let file_path = app_dir.join(format!("{}.json", context.commit));
    let json = serde_json::to_string_pretty(context)
        .map_err(|error| format!("Failed to serialize commit context: {error}"))?;

    fs::write(&file_path, json)
        .map_err(|error| format!("Failed to write {}: {}", file_path.display(), error))?;

    Ok(file_path)
}
