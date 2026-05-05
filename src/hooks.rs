use crate::git;
use std::fs;

const HOOK_MARKER_PREFIX: &str = "# gitwhisper post-commit hook";
const BEGIN_MARKER: &str = "# gitwhisper post-commit hook begin";
const END_MARKER: &str = "# gitwhisper post-commit hook end";

pub fn install_hook() {
    let hook_path = match git::git_dir() {
        Ok(git_dir) => git_dir.join("hooks").join("post-commit"),
        Err(error) => {
            eprintln!("Could not install hook: {error}");
            return;
        }
    };

    if let Some(parent) = hook_path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            eprintln!("Could not create hooks directory: {error}");
            return;
        }
    }

    let fallback_path = std::env::current_exe()
        .ok()
        .map(|path| path.display().to_string().replace('\\', "/"))
        .unwrap_or_else(|| "gitwhisper".to_string());

    let snippet = format!(
        r#"{begin}
if command -v gitwhisper >/dev/null 2>&1; then
  gitwhisper post-commit >/dev/null 2>&1 || true
else
  "{fallback}" post-commit >/dev/null 2>&1 || true
fi
{end}
"#,
        begin = BEGIN_MARKER,
        fallback = fallback_path,
        end = END_MARKER
    );

    let existing = fs::read_to_string(&hook_path).unwrap_or_default();
    let sanitized = strip_existing_gitwhisper_hook(&existing);

    let hook_contents = if sanitized.trim().is_empty() {
        format!("#!/bin/sh\n\n{snippet}")
    } else {
        format!("{}\n\n{}", sanitized.trim_end(), snippet)
    };

    if let Err(error) = fs::write(&hook_path, hook_contents) {
        eprintln!("Failed to write post-commit hook: {error}");
        return;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if let Ok(metadata) = fs::metadata(&hook_path) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o755);
            let _ = fs::set_permissions(&hook_path, permissions);
        }
    }

    println!(
        "Installed gitwhisper post-commit hook at {}",
        hook_path.display()
    );
}

fn strip_existing_gitwhisper_hook(existing: &str) -> String {
    if let (Some(begin), Some(end)) = (existing.find(BEGIN_MARKER), existing.find(END_MARKER)) {
        let after_end = end + END_MARKER.len();
        let mut cleaned = String::new();
        cleaned.push_str(existing[..begin].trim_end());
        if after_end < existing.len() {
            let remainder = existing[after_end..].trim_start_matches(['\r', '\n']);
            if !remainder.trim().is_empty() {
                if !cleaned.trim().is_empty() {
                    cleaned.push_str("\n\n");
                }
                cleaned.push_str(remainder);
            }
        }
        return cleaned;
    }

    if let Some(start) = existing.find(HOOK_MARKER_PREFIX) {
        if let Some(relative_end) = existing[start..].find("\nfi") {
            let end = start + relative_end + 3;
            let mut cleaned = String::new();
            cleaned.push_str(existing[..start].trim_end());
            if end < existing.len() {
                let remainder = existing[end..].trim_start_matches(['\r', '\n']);
                if !remainder.trim().is_empty() {
                    if !cleaned.trim().is_empty() {
                        cleaned.push_str("\n\n");
                    }
                    cleaned.push_str(remainder);
                }
            }
            return cleaned;
        }

        return existing[..start].trim_end().to_string();
    }

    existing.to_string()
}
