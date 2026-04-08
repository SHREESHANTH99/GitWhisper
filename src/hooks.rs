use crate::git;
use std::fs;

const HOOK_MARKER: &str = "# gitwhisper post-commit hook";

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
        r#"{marker}
if command -v gitwhisper >/dev/null 2>&1; then
  gitwhisper capture >/dev/null 2>&1 || true
else
  "{fallback}" capture >/dev/null 2>&1 || true
fi
"#,
        marker = HOOK_MARKER,
        fallback = fallback_path
    );

    let existing = fs::read_to_string(&hook_path).unwrap_or_default();
    if existing.contains(HOOK_MARKER) {
        println!("gitwhisper post-commit hook is already installed.");
        return;
    }

    let hook_contents = if existing.trim().is_empty() {
        format!("#!/bin/sh\n\n{snippet}")
    } else {
        format!("{}\n\n{}", existing.trim_end(), snippet)
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
