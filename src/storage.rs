pub mod cache_manager;
pub mod context;
pub mod load;
pub mod predictive_cache;
pub mod save;

use crate::error::AppResult;
use std::path::PathBuf;

pub fn app_dir() -> AppResult<PathBuf> {
    Ok(crate::git::git_dir()?.join("gitwhisper"))
}

pub fn legacy_app_dir() -> AppResult<PathBuf> {
    Ok(crate::git::git_dir()?.join("commitlens"))
}

pub fn cache_dir() -> AppResult<PathBuf> {
    Ok(app_dir()?.join("cache"))
}

pub fn log_dir() -> AppResult<PathBuf> {
    Ok(app_dir()?.join("logs"))
}

pub fn ai_log_path() -> AppResult<PathBuf> {
    Ok(log_dir()?.join("ai.log"))
}
