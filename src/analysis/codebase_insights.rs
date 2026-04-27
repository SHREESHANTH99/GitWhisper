use crate::analysis::ChangeCategory;
use crate::error::{AppError, AppResult};
use crate::history::HistoryEntry;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub const HISTORY_LIMIT: usize = 20;
const DUPLICATE_LINE_MIN_LEN: usize = 24;

#[derive(Debug, Clone)]
pub struct FileInsight {
    pub content: String,
    pub approx_loc: usize,
    pub approx_complexity: u32,
    pub duplicate_lines: usize,
    pub history: Vec<HistoryEntry>,
    pub recent_churn: usize,
    pub bug_fix_commits: usize,
    pub unique_authors: usize,
    pub top_owner_share: f64,
}

pub fn normalize_target(path: &str) -> AppResult<(std::path::PathBuf, String)> {
    let root = crate::git::repo_root()?;
    if path.trim().is_empty() || path.trim() == "." {
        return Ok((root, ".".to_string()));
    }

    let normalized = crate::git::normalize_repo_path(path)?;
    Ok((root, normalized))
}

pub fn collect_repo_files(root: &Path, directory: &Path) -> AppResult<Vec<String>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(directory).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() || is_hidden_or_git_path(path) || looks_binary(path) {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .map_err(|_| AppError::message("Could not resolve repository-relative path."))?;
        files.push(relative.to_string_lossy().replace('\\', "/"));
    }

    Ok(files)
}

pub fn collect_target_files(root: &Path, normalized: &str) -> AppResult<Vec<String>> {
    let absolute = if normalized == "." {
        root.to_path_buf()
    } else {
        root.join(normalized)
    };
    if absolute.is_file() {
        return Ok(vec![normalized.to_string()]);
    }

    if absolute.is_dir() {
        let files = collect_repo_files(root, &absolute)?;
        if files.is_empty() {
            return Err(AppError::message(format!(
                "No source files found under `{}`.",
                normalized
            )));
        }
        return Ok(files);
    }

    Err(AppError::message(format!(
        "Could not find `{}` inside the repository.",
        normalized
    )))
}

pub fn analyze_file(root: &Path, normalized_file: &str) -> AppResult<FileInsight> {
    let absolute = root.join(normalized_file);
    let content = fs::read_to_string(&absolute).map_err(|_| {
        AppError::message(format!(
            "Could not read `{}` as text for analysis.",
            normalized_file
        ))
    })?;

    let history = crate::history::load_history_for_file(normalized_file, HISTORY_LIMIT)?;
    let owners = crate::git::owners_for_path(normalized_file, 10).unwrap_or_default();

    Ok(FileInsight {
        approx_loc: content.lines().filter(|line| !line.trim().is_empty()).count(),
        approx_complexity: estimate_complexity(&content),
        duplicate_lines: count_duplicate_lines(&content),
        recent_churn: recent_churn(&history, normalized_file),
        bug_fix_commits: bug_fix_count(&history),
        unique_authors: owners.len(),
        top_owner_share: top_owner_share(&owners),
        history,
        content,
    })
}

pub fn estimate_complexity(content: &str) -> u32 {
    let mut complexity = 1u32;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with('*')
            || trimmed.is_empty()
        {
            continue;
        }

        complexity += keyword_hits(trimmed, &["if ", "else if", "match ", "case ", "when "]) as u32;
        complexity += keyword_hits(trimmed, &["for ", "while ", "loop ", "catch ", "except "]) as u32;
        complexity += keyword_hits(trimmed, &["&&", "||"]) as u32;

        if trimmed.contains('?') && !trimmed.starts_with("///") {
            complexity += 1;
        }
    }

    complexity
}

pub fn count_duplicate_lines(content: &str) -> usize {
    let mut counts: HashMap<String, usize> = HashMap::new();

    for line in content.lines() {
        let normalized = normalize_code_line(line);
        if normalized.len() < DUPLICATE_LINE_MIN_LEN {
            continue;
        }

        *counts.entry(normalized).or_insert(0) += 1;
    }

    counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(_, count)| count)
        .sum()
}

pub fn normalize_code_line(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn recent_churn(history: &[HistoryEntry], normalized_file: &str) -> usize {
    history
        .iter()
        .filter_map(|entry| entry.context.as_ref())
        .flat_map(|context| context.analysis.diff.file_stats.iter())
        .filter(|stat| stat.path == normalized_file)
        .map(|stat| stat.added + stat.removed)
        .sum()
}

pub fn bug_fix_count(history: &[HistoryEntry]) -> usize {
    history.iter().filter(|entry| is_bug_fix(entry)).count()
}

pub fn is_bug_fix(entry: &HistoryEntry) -> bool {
    if let Some(context) = &entry.context {
        if context.analysis.intent.category == ChangeCategory::BugFix {
            return true;
        }
    }

    let mut text = entry.commit.subject.to_ascii_lowercase();
    if !entry.commit.body.trim().is_empty() {
        text.push('\n');
        text.push_str(&entry.commit.body.to_ascii_lowercase());
    }

    ["fix", "bug", "hotfix", "patch"]
        .iter()
        .any(|needle| text.contains(needle))
}

pub fn top_owner_share(owners: &[crate::git::OwnerStat]) -> f64 {
    let total: usize = owners.iter().map(|owner| owner.commits).sum();
    if total == 0 {
        return 0.0;
    }

    owners
        .iter()
        .map(|owner| owner.commits as f64 / total as f64)
        .fold(0.0, f64::max)
}

fn is_hidden_or_git_path(path: &Path) -> bool {
    path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy();
        value == ".git" || value.starts_with('.')
    })
}

fn looks_binary(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    matches!(
        extension.as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "bmp"
            | "ico"
            | "lock"
            | "pdf"
            | "exe"
            | "dll"
            | "so"
            | "dylib"
            | "zip"
            | "tar"
            | "gz"
            | "7z"
            | "woff"
            | "woff2"
            | "ttf"
            | "eot"
    )
}

fn keyword_hits(line: &str, needles: &[&str]) -> usize {
    needles.iter().map(|needle| line.matches(needle).count()).sum()
}

#[cfg(test)]
mod tests {
    use super::{count_duplicate_lines, estimate_complexity, normalize_code_line};

    #[test]
    fn normalizes_duplicate_lines() {
        assert_eq!(
            normalize_code_line(" let   token = validate ( input ) ; "),
            "let token = validate ( input ) ;"
        );
    }

    #[test]
    fn detects_duplicate_lines() {
        let content = r#"
let token = validate_user_session(current_user, token_store);
let token = validate_user_session(current_user, token_store);
let token = validate_user_session(current_user, token_store);
"#;

        assert!(count_duplicate_lines(content) >= 3);
    }

    #[test]
    fn estimates_complexity_from_branching() {
        let content = r#"
fn validate(input: bool, other: bool) {
    if input && other {
        for item in 0..10 {
            if item > 1 || other {
                println!("{}", item);
            }
        }
    } else if other {
        match input {
            true => println!("yes"),
            false => println!("no"),
        }
    }
}
"#;

        assert!(estimate_complexity(content) >= 8);
    }
}
