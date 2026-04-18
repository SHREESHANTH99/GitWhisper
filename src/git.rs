use crate::error::{AppError, AppResult};
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

#[derive(Debug, Clone)]
pub struct CommitMessage {
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct AuthorCommitRecord {
    pub timestamp: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OwnerStat {
    pub commits: usize,
    pub name: String,
    pub email: String,
}

pub fn repo_root() -> AppResult<PathBuf> {
    let output = run_git(&["rev-parse", "--show-toplevel"])?;
    Ok(PathBuf::from(output))
}

pub fn git_dir() -> AppResult<PathBuf> {
    let root = repo_root()?;
    let output = run_git(&["rev-parse", "--git-dir"])?;
    let git_dir = PathBuf::from(output);

    if git_dir.is_absolute() {
        Ok(git_dir)
    } else {
        Ok(root.join(git_dir))
    }
}

pub fn short_commit_hash() -> AppResult<String> {
    run_git(&["rev-parse", "--short=7", "HEAD"])
}

pub fn head_commit_hash() -> AppResult<String> {
    run_git(&["rev-parse", "HEAD"])
}

pub fn resolve_commit(commitish: &str) -> AppResult<String> {
    run_git(&["rev-parse", commitish])
}

pub fn short_commit_hash_of(commitish: &str) -> AppResult<String> {
    let short_arg = format!("--short=7");
    run_git(&["rev-parse", &short_arg, commitish])
}

pub fn current_branch() -> AppResult<String> {
    run_git(&["rev-parse", "--abbrev-ref", "HEAD"])
}

pub fn remote_url(name: &str) -> AppResult<String> {
    let key = format!("remote.{name}.url");
    run_git(&["config", "--get", &key])
}

pub fn commit_subject(commit: &str) -> AppResult<String> {
    Ok(commit_message(commit)?.subject)
}

pub fn commit_author_email(commit: &str) -> AppResult<String> {
    run_git(&["show", "-s", "--format=%ae", commit])
}

pub fn commit_author_name(commit: &str) -> AppResult<String> {
    run_git(&["show", "-s", "--format=%an", commit])
}

pub fn commit_message(commit: &str) -> AppResult<CommitMessage> {
    let output = run_git(&["show", "-s", "--format=%s%x1f%b", commit])?;
    let (subject, body) = output.split_once('\u{1f}').unwrap_or((output.as_str(), ""));

    Ok(CommitMessage {
        subject: subject.trim().to_string(),
        body: body.trim().to_string(),
    })
}

pub fn author_history(author_email: &str, max_count: usize) -> AppResult<Vec<AuthorCommitRecord>> {
    let author_arg = format!("--author={author_email}");
    let max_count_arg = format!("--max-count={max_count}");
    let output = run_git(&[
        "log",
        "--all",
        &author_arg,
        &max_count_arg,
        "--format=%cI%x1e",
        "--name-only",
    ])?;

    Ok(parse_author_history(&output))
}

pub fn changed_files_for_commit(commit: &str) -> AppResult<Vec<String>> {
    let output = run_git(&["show", "--pretty=format:", "--name-only", commit, "--"])?;
    Ok(output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.replace('\\', "/"))
        .collect())
}

pub fn commit_patch(commit: &str) -> AppResult<String> {
    run_git(&[
        "show",
        "--find-renames",
        "--find-copies",
        "--format=",
        "--patch",
        "--unified=0",
        commit,
        "--",
    ])
}

pub fn file_history(file: &str, limit: usize) -> AppResult<Vec<FileCommit>> {
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

pub fn owners_for_path(path: &str, limit: usize) -> AppResult<Vec<OwnerStat>> {
    let normalized = normalize_repo_path(path)?;
    let args = vec!["shortlog", "-sne", "HEAD", "--", normalized.as_str()];
    let output = run_git(&args)?;
    Ok(parse_shortlog(&output, limit))
}

pub fn add_git_note(commit: &str, note_ref: &str, message: &str) -> AppResult<()> {
    let ref_arg = format!("--ref={note_ref}");
    let args = vec!["notes", ref_arg.as_str(), "add", "-f", "-m", message, commit];
    run_git(&args).map(|_| ())
}

pub fn normalize_repo_path(path: &str) -> AppResult<String> {
    let root = repo_root()?;
    let provided = PathBuf::from(path);
    let relative = if provided.is_absolute() {
        provided
            .strip_prefix(&root)
            .map(Path::to_path_buf)
            .map_err(|_| AppError::message("Please provide a file path inside the repository."))?
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
        Err(AppError::message(
            "Please provide a file path inside the repository.",
        ))
    } else {
        Ok(normalized)
    }
}

fn parse_shortlog(raw: &str, limit: usize) -> Vec<OwnerStat> {
    let mut stats = raw
        .lines()
        .filter_map(|line| parse_shortlog_line(line))
        .collect::<Vec<_>>();

    stats.sort_by(|a, b| b.commits.cmp(&a.commits));

    if limit > 0 && stats.len() > limit {
        stats.truncate(limit);
    }

    stats
}

fn parse_shortlog_line(line: &str) -> Option<OwnerStat> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut parts = trimmed.split_whitespace();
    let commits: usize = parts.next()?.parse().ok()?;
    let rest = parts.collect::<Vec<_>>().join(" ");
    let (name, email) = parse_name_email(&rest);

    Some(OwnerStat {
        commits,
        name,
        email,
    })
}

fn parse_name_email(input: &str) -> (String, String) {
    let trimmed = input.trim();
    let Some(start) = trimmed.rfind('<') else {
        return (trimmed.to_string(), String::new());
    };
    let Some(end) = trimmed.rfind('>') else {
        return (trimmed.to_string(), String::new());
    };

    if start >= end {
        return (trimmed.to_string(), String::new());
    }

    let name = trimmed[..start].trim().to_string();
    let email = trimmed[start + 1..end].trim().to_string();
    (name, email)
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

fn parse_author_history(raw: &str) -> Vec<AuthorCommitRecord> {
    raw.split('\u{1e}')
        .filter_map(|record| {
            let trimmed = record.trim();
            if trimmed.is_empty() {
                return None;
            }

            let mut lines = trimmed.lines();
            let timestamp = lines.next()?.trim().to_string();
            let files = lines
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(|line| line.replace('\\', "/"))
                .collect::<Vec<_>>();

            Some(AuthorCommitRecord { timestamp, files })
        })
        .collect()
}

fn run_git(args: &[&str]) -> AppResult<String> {
    let output = Command::new("git").args(args).output().map_err(|error| {
        AppError::Git(format!("Failed to run git {}: {}", args.join(" "), error))
    })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(AppError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}
