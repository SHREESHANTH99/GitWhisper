use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct FileCommit {
    pub hash: String,
    pub short_hash: String,
    pub timestamp: String,
    pub subject: String,
    pub body: String,
}

pub fn repo_root() -> Result<PathBuf, String> {
    let output = run_git(&["rev-parse", "--show-toplevel"])?;
    Ok(PathBuf::from(output))
}

pub fn git_dir() -> Result<PathBuf, String> {
    let root = repo_root()?;
    let output = run_git(&["rev-parse", "--git-dir"])?;
    let git_dir = PathBuf::from(output);

    if git_dir.is_absolute() {
        Ok(git_dir)
    } else {
        Ok(root.join(git_dir))
    }
}

pub fn short_commit_hash() -> Option<String> {
    run_git(&["rev-parse", "--short=7", "HEAD"]).ok()
}

pub fn current_branch() -> Option<String> {
    run_git(&["rev-parse", "--abbrev-ref", "HEAD"]).ok()
}

pub fn commit_subject(commit: &str) -> Option<String> {
    run_git(&["show", "-s", "--format=%s", commit]).ok()
}

pub fn changed_files_for_commit(commit: &str) -> Result<Vec<String>, String> {
    let output = run_git(&["show", "--pretty=format:", "--name-only", commit, "--"])?;
    Ok(output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.replace('\\', "/"))
        .collect())
}

pub fn file_history(file: &str, limit: usize) -> Result<Vec<FileCommit>, String> {
    let normalized = normalize_repo_path(file)?;
    let format = "%H%x1f%h%x1f%cI%x1f%s%x1f%b%x1e";
    let max_count = format!("--max-count={limit}");
    let format_arg = format!("--format={format}");
    let output = run_git(&[
        "log",
        "--follow",
        &max_count,
        &format_arg,
        "--",
        &normalized,
    ])?;
    Ok(parse_history(&output))
}

pub fn normalize_repo_path(path: &str) -> Result<String, String> {
    let root = repo_root()?;
    let provided = PathBuf::from(path);
    let relative = if provided.is_absolute() {
        provided
            .strip_prefix(&root)
            .map(Path::to_path_buf)
            .unwrap_or(provided)
    } else {
        provided
    };

    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => parts.push(part.to_string_lossy().to_string()),
            Component::ParentDir => parts.push("..".to_string()),
            Component::Prefix(prefix) => {
                parts.push(prefix.as_os_str().to_string_lossy().to_string())
            }
            Component::RootDir => {}
        }
    }

    let normalized = parts.join("/");
    if normalized.is_empty() {
        Err("Please provide a file path inside the repository.".to_string())
    } else {
        Ok(normalized)
    }
}

fn parse_history(raw: &str) -> Vec<FileCommit> {
    raw.split('\u{1e}')
        .filter_map(|record| {
            let trimmed = record.trim();
            if trimmed.is_empty() {
                return None;
            }

            let mut fields = trimmed.split('\u{1f}');
            Some(FileCommit {
                hash: fields.next()?.trim().to_string(),
                short_hash: fields.next()?.trim().to_string(),
                timestamp: fields.next()?.trim().to_string(),
                subject: fields.next()?.trim().to_string(),
                body: fields.next().unwrap_or_default().trim().to_string(),
            })
        })
        .collect()
}

fn run_git(args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|error| format!("Failed to run git {}: {}", args.join(" "), error))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}
