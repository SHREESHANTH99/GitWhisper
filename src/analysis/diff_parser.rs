use crate::analysis::{
    ChangeKind, DiffSummary, FileDiffStat, FileOperation, ImportChange, SymbolChange, SymbolKind,
};
use crate::error::AppResult;

pub fn summarize_commit(commit: &str) -> AppResult<DiffSummary> {
    let patch = crate::git::commit_patch(commit)?;
    Ok(parse_patch(&patch))
}

fn parse_patch(raw: &str) -> DiffSummary {
    let mut summary = DiffSummary::default();
    let mut current_file: Option<FileDiffStat> = None;

    for line in raw.lines() {
        if line.starts_with("diff --git ") {
            if let Some(file_stat) = current_file.take() {
                push_file_stat(&mut summary, file_stat);
            }

            current_file = Some(start_file_stat(line));
            continue;
        }

        let Some(file_stat) = current_file.as_mut() else {
            continue;
        };

        if line.starts_with("new file mode ") {
            file_stat.operation = FileOperation::Added;
            continue;
        }

        if line.starts_with("deleted file mode ") {
            file_stat.operation = FileOperation::Deleted;
            continue;
        }

        if let Some(path) = line.strip_prefix("rename from ") {
            file_stat.previous_path = Some(normalize_diff_path(path));
            file_stat.operation = FileOperation::Renamed;
            continue;
        }

        if let Some(path) = line.strip_prefix("rename to ") {
            file_stat.path = normalize_diff_path(path);
            file_stat.language = detect_language(&file_stat.path);
            file_stat.operation = FileOperation::Renamed;
            continue;
        }

        if let Some(path) = line.strip_prefix("--- ") {
            if path.trim() == "/dev/null" {
                file_stat.operation = FileOperation::Added;
            } else if file_stat.previous_path.is_none() {
                file_stat.previous_path = Some(normalize_diff_path(path));
            }
            continue;
        }

        if let Some(path) = line.strip_prefix("+++ ") {
            if path.trim() == "/dev/null" {
                file_stat.operation = FileOperation::Deleted;
            } else {
                file_stat.path = normalize_diff_path(path);
                file_stat.language = detect_language(&file_stat.path);
            }
            continue;
        }

        if line.starts_with("@@") {
            handle_hunk_context(file_stat, line);
            continue;
        }

        if line.starts_with('+') && !line.starts_with("+++") {
            handle_changed_line(file_stat, &line[1..], ChangeKind::Added);
            continue;
        }

        if line.starts_with('-') && !line.starts_with("---") {
            handle_changed_line(file_stat, &line[1..], ChangeKind::Removed);
        }
    }

    if let Some(file_stat) = current_file.take() {
        push_file_stat(&mut summary, file_stat);
    }

    summary.files_changed = summary.file_stats.len();
    summary.net_lines = summary.lines_added as isize - summary.lines_removed as isize;
    summary
}

fn start_file_stat(header: &str) -> FileDiffStat {
    let mut parts = header.split_whitespace();
    let _ = parts.next();
    let _ = parts.next();
    let old_path = parts.next().map(normalize_diff_path);
    let new_path = parts.next().map(normalize_diff_path);

    let path = new_path
        .filter(|path| !path.is_empty())
        .or_else(|| old_path.clone())
        .unwrap_or_default();

    FileDiffStat {
        path: path.clone(),
        previous_path: old_path,
        operation: FileOperation::Modified,
        language: detect_language(&path),
        ..FileDiffStat::default()
    }
}

fn push_file_stat(summary: &mut DiffSummary, mut file_stat: FileDiffStat) {
    collapse_symbol_changes(&mut file_stat.symbol_changes);
    dedupe_import_changes(&mut file_stat.import_changes);

    summary.lines_added += file_stat.added;
    summary.lines_removed += file_stat.removed;
    summary.complexity_delta += file_stat.complexity_delta;
    summary
        .import_changes
        .extend(file_stat.import_changes.clone());
    summary
        .symbol_changes
        .extend(file_stat.symbol_changes.clone());

    match file_stat.operation {
        FileOperation::Added => summary.files_added += 1,
        FileOperation::Deleted => summary.files_deleted += 1,
        FileOperation::Renamed => summary.files_renamed += 1,
        FileOperation::Copied | FileOperation::Modified => {}
    }

    summary.file_stats.push(file_stat);
}

fn handle_hunk_context(file_stat: &mut FileDiffStat, hunk_header: &str) {
    let Some((_, context)) = hunk_header.rsplit_once("@@") else {
        return;
    };

    let context = context.trim();
    let Some(symbol_change) = build_symbol_change(
        &file_stat.path,
        context,
        &file_stat.language,
        ChangeKind::Modified,
    ) else {
        return;
    };

    file_stat.symbol_changes.push(symbol_change);
}

fn handle_changed_line(file_stat: &mut FileDiffStat, line: &str, kind: ChangeKind) {
    let complexity_score = complexity_score(line) as isize;

    match kind {
        ChangeKind::Added => {
            file_stat.added += 1;
            file_stat.complexity_delta += complexity_score;
        }
        ChangeKind::Removed => {
            file_stat.removed += 1;
            file_stat.complexity_delta -= complexity_score;
        }
        ChangeKind::Modified => {}
    }

    if let Some(statement) = detect_import_statement(line, &file_stat.language) {
        file_stat.import_changes.push(ImportChange {
            file_path: file_stat.path.clone(),
            statement,
            kind: kind.clone(),
        });
    }

    if let Some(symbol_change) =
        build_symbol_change(&file_stat.path, line, &file_stat.language, kind)
    {
        file_stat.symbol_changes.push(symbol_change);
    }
}

fn build_symbol_change(
    file_path: &str,
    line: &str,
    language: &str,
    kind: ChangeKind,
) -> Option<SymbolChange> {
    let (symbol_kind, symbol_name, signature) = detect_symbol(line, language)?;

    Some(SymbolChange {
        file_path: file_path.to_string(),
        symbol_name,
        signature,
        kind,
        symbol_kind,
    })
}

fn detect_import_statement(line: &str, language: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || is_comment_line(trimmed, language) {
        return None;
    }

    let is_import = trimmed.starts_with("use ")
        || trimmed.starts_with("pub use ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("from ")
        || trimmed.starts_with("require(")
        || trimmed.contains(" require(")
        || (trimmed.starts_with("export ") && trimmed.contains(" from "))
        || trimmed.starts_with("mod ")
        || trimmed.starts_with("pub mod ")
        || trimmed.starts_with("#include ")
        || trimmed.starts_with("include ");

    is_import.then(|| compact_text(trimmed, 100))
}

fn detect_symbol(line: &str, language: &str) -> Option<(SymbolKind, String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || is_comment_line(trimmed, language) {
        return None;
    }

    let signature = compact_text(trimmed, 140);
    match language {
        "rust" => detect_rust_symbol(trimmed).map(|(kind, name)| (kind, name, signature)),
        "javascript" | "typescript" => {
            detect_javascript_symbol(trimmed).map(|(kind, name)| (kind, name, signature))
        }
        "python" => detect_python_symbol(trimmed).map(|(kind, name)| (kind, name, signature)),
        _ => detect_generic_symbol(trimmed).map(|(kind, name)| (kind, name, signature)),
    }
}

fn detect_rust_symbol(line: &str) -> Option<(SymbolKind, String)> {
    extract_name_after_keyword(line, "fn ")
        .map(|name| (SymbolKind::Function, name))
        .or_else(|| {
            extract_name_after_keyword(line, "struct ").map(|name| (SymbolKind::Type, name))
        })
        .or_else(|| extract_name_after_keyword(line, "enum ").map(|name| (SymbolKind::Type, name)))
        .or_else(|| extract_name_after_keyword(line, "trait ").map(|name| (SymbolKind::Type, name)))
        .or_else(|| extract_name_after_keyword(line, "type ").map(|name| (SymbolKind::Type, name)))
        .or_else(|| extract_name_after_keyword(line, "mod ").map(|name| (SymbolKind::Module, name)))
}

fn detect_javascript_symbol(line: &str) -> Option<(SymbolKind, String)> {
    extract_name_after_keyword(line, "function ")
        .map(|name| (SymbolKind::Function, name))
        .or_else(|| extract_name_before_arrow(line).map(|name| (SymbolKind::Function, name)))
        .or_else(|| extract_name_after_keyword(line, "class ").map(|name| (SymbolKind::Type, name)))
        .or_else(|| {
            extract_name_after_keyword(line, "interface ").map(|name| (SymbolKind::Type, name))
        })
        .or_else(|| extract_name_after_keyword(line, "type ").map(|name| (SymbolKind::Type, name)))
}

fn detect_python_symbol(line: &str) -> Option<(SymbolKind, String)> {
    extract_name_after_keyword(line, "def ")
        .map(|name| (SymbolKind::Function, name))
        .or_else(|| extract_name_after_keyword(line, "class ").map(|name| (SymbolKind::Type, name)))
}

fn detect_generic_symbol(line: &str) -> Option<(SymbolKind, String)> {
    extract_name_after_keyword(line, "fn ")
        .map(|name| (SymbolKind::Function, name))
        .or_else(|| {
            extract_name_after_keyword(line, "def ").map(|name| (SymbolKind::Function, name))
        })
        .or_else(|| {
            extract_name_after_keyword(line, "function ").map(|name| (SymbolKind::Function, name))
        })
        .or_else(|| extract_name_before_arrow(line).map(|name| (SymbolKind::Function, name)))
        .or_else(|| extract_name_after_keyword(line, "class ").map(|name| (SymbolKind::Type, name)))
        .or_else(|| {
            extract_name_after_keyword(line, "module ").map(|name| (SymbolKind::Module, name))
        })
        .or_else(|| extract_name_after_keyword(line, "mod ").map(|name| (SymbolKind::Module, name)))
}

fn extract_name_after_keyword(line: &str, keyword: &str) -> Option<String> {
    let start = line.find(keyword)? + keyword.len();
    let candidate = &line[start..];
    let name = candidate
        .chars()
        .skip_while(|character| character.is_whitespace())
        .take_while(|character| {
            character.is_alphanumeric() || *character == '_' || *character == '$'
        })
        .collect::<String>();

    (!name.is_empty()).then_some(name)
}

fn extract_name_before_arrow(line: &str) -> Option<String> {
    if !line.contains("=>") || !line.contains('=') {
        return None;
    }

    let before_equals = line.split('=').next()?.trim_end();
    let candidate = before_equals
        .split_whitespace()
        .last()
        .unwrap_or_default()
        .trim_matches(|character: char| character == '(' || character == ')' || character == '{');

    let is_identifier = !candidate.is_empty()
        && candidate
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_' || character == '$');

    is_identifier.then(|| candidate.to_string())
}

fn collapse_symbol_changes(changes: &mut Vec<SymbolChange>) {
    let mut collapsed = Vec::new();
    let mut consumed = vec![false; changes.len()];

    for index in 0..changes.len() {
        if consumed[index] {
            continue;
        }

        let mut change = changes[index].clone();
        for other_index in (index + 1)..changes.len() {
            if consumed[other_index] {
                continue;
            }

            let other = &changes[other_index];
            let same_symbol = change.file_path == other.file_path
                && change.symbol_name == other.symbol_name
                && change.symbol_kind == other.symbol_kind;

            if !same_symbol || change.kind == other.kind {
                continue;
            }

            consumed[other_index] = true;
            change.kind = ChangeKind::Modified;
            if other.kind == ChangeKind::Added {
                change.signature = other.signature.clone();
            }
        }

        if !collapsed.contains(&change) {
            collapsed.push(change);
        }
    }

    *changes = collapsed;
}

fn dedupe_import_changes(changes: &mut Vec<ImportChange>) {
    let mut deduped = Vec::new();
    for change in changes.drain(..) {
        if !deduped.contains(&change) {
            deduped.push(change);
        }
    }
    *changes = deduped;
}

fn complexity_score(line: &str) -> usize {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return 0;
    }

    let signals = [
        "if ", "else if", "elif ", "for ", "while ", "match ", "case ", "catch ", "&&", "||",
    ];

    signals
        .iter()
        .filter(|signal| trimmed.contains(*signal))
        .count()
}

fn detect_language(path: &str) -> String {
    let extension = path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "rs" => "rust",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" => "typescript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "c" | "cc" | "cpp" | "h" | "hpp" => "c-family",
        "md" | "mdx" | "txt" => "text",
        _ => "unknown",
    }
    .to_string()
}

fn normalize_diff_path(path: &str) -> String {
    let trimmed = path.trim().trim_matches('"');
    let without_prefix = trimmed
        .strip_prefix("a/")
        .or_else(|| trimmed.strip_prefix("b/"))
        .unwrap_or(trimmed);

    without_prefix.replace('\\', "/")
}

fn is_comment_line(line: &str, language: &str) -> bool {
    line.starts_with("//")
        || line.starts_with("/*")
        || line.starts_with('*')
        || (language == "python" && line.starts_with('#'))
}

fn compact_text(input: &str, max_len: usize) -> String {
    let collapsed = input.split_whitespace().collect::<Vec<_>>().join(" ");
    let collapsed_len = collapsed.chars().count();

    if collapsed_len <= max_len {
        collapsed
    } else if max_len <= 3 {
        ".".repeat(max_len)
    } else {
        let prefix = collapsed.chars().take(max_len - 3).collect::<String>();
        format!("{prefix}...")
    }
}

#[cfg(test)]
mod tests {
    use super::parse_patch;
    use crate::analysis::{ChangeKind, FileOperation, SymbolKind};

    #[test]
    fn parses_semantic_diff_signals_from_patch() {
        let patch = r#"
diff --git a/src/auth.rs b/src/auth.rs
index 1111111..2222222 100644
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -1,2 +1,4 @@
-use crate::auth::Session;
+use crate::auth::jwt::Token;
@@ -10,1 +12,4 @@ fn validate_user(token: &str) -> bool {
-fn validate_user(token: &str) -> bool {
+fn validate_user(token: &str, issuer: &str) -> bool {
+    if issuer.is_empty() || token.is_empty() {
+        return false;
+    }
diff --git a/src/legacy.rs b/src/core.rs
similarity index 100%
rename from src/legacy.rs
rename to src/core.rs
diff --git a/web/client.ts b/web/client.ts
new file mode 100644
--- /dev/null
+++ b/web/client.ts
@@ -0,0 +1,3 @@
+import { http } from './http';
+export const buildClient = () => http();
+if (http) { console.log('ready'); }
"#;

        let summary = parse_patch(patch);

        assert_eq!(summary.files_changed, 3);
        assert_eq!(summary.files_added, 1);
        assert_eq!(summary.files_renamed, 1);
        assert_eq!(summary.files_deleted, 0);
        assert_eq!(summary.lines_added, 8);
        assert_eq!(summary.lines_removed, 2);
        assert!(summary.complexity_delta > 0);
        assert_eq!(summary.import_changes.len(), 3);
        assert!(summary
            .symbol_changes
            .iter()
            .any(|change| change.symbol_name == "validate_user"
                && change.kind == ChangeKind::Modified
                && change.symbol_kind == SymbolKind::Function));
        assert!(summary
            .symbol_changes
            .iter()
            .any(|change| change.symbol_name == "buildClient"
                && change.kind == ChangeKind::Added
                && change.symbol_kind == SymbolKind::Function));
        assert!(summary
            .file_stats
            .iter()
            .any(|file| file.operation == FileOperation::Renamed && file.path == "src/core.rs"));
    }

    #[test]
    fn keeps_hunk_context_as_modified_symbol_signal() {
        let patch = r#"
diff --git a/src/service.py b/src/service.py
index 1234567..89abcde 100644
--- a/src/service.py
+++ b/src/service.py
@@ -10,2 +10,2 @@ def process_user(user_id):
-    return cache[user_id]
+    return cache.get(user_id)
"#;

        let summary = parse_patch(patch);

        assert!(summary
            .symbol_changes
            .iter()
            .any(|change| change.symbol_name == "process_user"
                && change.kind == ChangeKind::Modified));
    }
}
