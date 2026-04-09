use crate::storage::context::IdeContext;
use std::fs;
use std::process::Command;

pub fn collect_ide_context(candidate_files: &[String]) -> IdeContext {
    let processes = running_processes();
    let (name, process) = detect_ide(&processes);
    let build_system = infer_build_system();
    let extensions = detect_extensions(&name);

    IdeContext {
        name,
        process,
        version: String::new(),
        build_system,
        extensions,
        active_files: candidate_files.iter().take(5).cloned().collect(),
    }
}

fn running_processes() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("tasklist")
            .args(["/fo", "csv", "/nh"])
            .output()
            .ok();
        let Some(output) = output else {
            return Vec::new();
        };

        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.split(',').next())
            .map(|entry| entry.trim_matches('"').to_ascii_lowercase())
            .collect()
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("ps").args(["-A", "-o", "comm="]).output().ok();
        let Some(output) = output else {
            return Vec::new();
        };

        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| line.trim().to_ascii_lowercase())
            .filter(|line| !line.is_empty())
            .collect()
    }
}

fn detect_ide(processes: &[String]) -> (String, String) {
    let known = [
        ("cursor", "Cursor"),
        ("code.exe", "VS Code"),
        ("code-insiders", "VS Code Insiders"),
        ("idea64.exe", "IntelliJ IDEA"),
        ("pycharm64.exe", "PyCharm"),
        ("webstorm64.exe", "WebStorm"),
        ("clion64.exe", "CLion"),
        ("devenv.exe", "Visual Studio"),
        ("nvim", "Neovim"),
        ("vim", "Vim"),
    ];

    for process in processes {
        for (needle, label) in known {
            if process.contains(needle) {
                return (label.to_string(), process.clone());
            }
        }
    }

    (String::new(), String::new())
}

fn infer_build_system() -> String {
    let Ok(root) = crate::git::repo_root() else {
        return String::new();
    };

    let candidates = [
        ("Cargo.toml", "cargo"),
        ("package.json", "npm"),
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
        ("pyproject.toml", "python"),
        ("requirements.txt", "python"),
        ("pom.xml", "maven"),
    ];

    for (file, build_system) in candidates {
        if root.join(file).exists() {
            return build_system.to_string();
        }
    }

    String::new()
}

fn detect_extensions(ide_name: &str) -> Vec<String> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };

    let directories = if ide_name == "Cursor" {
        vec![home.join(".cursor").join("extensions")]
    } else if ide_name.contains("VS Code") {
        vec![home.join(".vscode").join("extensions")]
    } else {
        Vec::new()
    };

    directories
        .into_iter()
        .flat_map(|directory| fs::read_dir(directory).into_iter().flatten().flatten())
        .filter_map(|entry| entry.file_name().to_str().map(|name| name.to_string()))
        .take(5)
        .collect()
}
