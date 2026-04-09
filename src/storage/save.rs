use crate::error::AppResult;
use crate::storage::context::CommitContext;
use std::fs;
use std::path::PathBuf;

pub fn save_context(context: &CommitContext) -> AppResult<PathBuf> {
    let app_dir = crate::storage::app_dir()?;
    fs::create_dir_all(&app_dir)?;

    let file_path = app_dir.join(format!("{}.json", context.commit));
    let json = serde_json::to_string_pretty(context)?;

    fs::write(&file_path, json)?;

    Ok(file_path)
}
