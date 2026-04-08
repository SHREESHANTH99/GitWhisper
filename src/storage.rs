pub mod context;
pub mod load;
pub mod save;

use std::path::PathBuf;

pub fn app_dir() -> Result<PathBuf, String> {
    Ok(crate::git::git_dir()?.join("gitwhisper"))
}

pub fn legacy_app_dir() -> Result<PathBuf, String> {
    Ok(crate::git::git_dir()?.join("commitlens"))
}

pub fn cache_dir() -> Result<PathBuf, String> {
    Ok(app_dir()?.join("cache"))
}

pub fn log_dir() -> Result<PathBuf, String> {
    Ok(app_dir()?.join("logs"))
}

pub fn ai_log_path() -> Result<PathBuf, String> {
    Ok(log_dir()?.join("ai.log"))
}
