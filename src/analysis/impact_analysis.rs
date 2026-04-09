use crate::analysis::{DiffSummary, ImpactSummary};
use crate::error::AppResult;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn analyze_impact(changed_files: &[String], diff: &DiffSummary) -> AppResult<ImpactSummary> {
    let root = crate::git::repo_root()?;
    let graph = build_dependency_graph(&root);
    let reverse = reverse_graph(&graph);

    let changed = changed_files
        .iter()
        .map(|file| normalize_path(file))
        .collect::<HashSet<_>>();

    let direct_dependents = collect_dependents(&changed, &reverse, 1);
    let transitive_dependents = collect_dependents(&changed, &reverse, 3)
        .into_iter()
        .filter(|path| !direct_dependents.contains(path))
        .collect::<BTreeSet<_>>();

    let relevant = changed
        .iter()
        .cloned()
        .chain(direct_dependents.iter().cloned())
        .chain(transitive_dependents.iter().cloned())
        .collect::<HashSet<_>>();
    let circular_dependencies = detect_cycles(&graph, &relevant, 4);

    let impact_score = score_impact(
        direct_dependents.len(),
        transitive_dependents.len(),
        circular_dependencies.len(),
        diff,
    );

    Ok(ImpactSummary {
        impact_score,
        direct_dependents: direct_dependents.into_iter().collect(),
        transitive_dependents: transitive_dependents.into_iter().collect(),
        circular_dependencies,
    })
}

fn build_dependency_graph(root: &Path) -> HashMap<String, BTreeSet<String>> {
    let mut graph = HashMap::new();
    let repo_files = collect_repo_files(root);
    let file_set = repo_files.iter().cloned().collect::<HashSet<_>>();

    for relative_path in repo_files {
        let absolute_path = root.join(&relative_path);
        let Ok(content) = fs::read_to_string(&absolute_path) else {
            continue;
        };

        let imports = parse_dependencies(root, &relative_path, &content, &file_set);
        graph.insert(relative_path, imports.into_iter().collect());
    }

    graph
}

fn collect_repo_files(root: &Path) -> Vec<String> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let absolute = entry.path();
            let relative = absolute.strip_prefix(root).ok()?;
            let normalized = normalize_path(&relative.to_string_lossy());

            if normalized.starts_with(".git/")
                || normalized.starts_with("target/")
                || normalized.starts_with("node_modules/")
            {
                return None;
            }

            is_supported_source_file(&normalized).then_some(normalized)
        })
        .collect()
}

fn parse_dependencies(
    root: &Path,
    relative_path: &str,
    content: &str,
    file_set: &HashSet<String>,
) -> HashSet<String> {
    let mut dependencies = HashSet::new();
    let extension = file_extension(relative_path);

    for line in content.lines() {
        match extension {
            "rs" => parse_rust_dependency(relative_path, line, file_set, &mut dependencies),
            "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" => {
                parse_script_dependency(root, relative_path, line, file_set, &mut dependencies)
            }
            "py" => parse_python_dependency(relative_path, line, file_set, &mut dependencies),
            _ => {}
        }
    }

    dependencies
}

fn parse_rust_dependency(
    relative_path: &str,
    line: &str,
    file_set: &HashSet<String>,
    dependencies: &mut HashSet<String>,
) {
    let trimmed = line.trim();
    if trimmed.starts_with("use crate::") || trimmed.starts_with("pub use crate::") {
        let path = trimmed
            .split("crate::")
            .nth(1)
            .unwrap_or_default()
            .split(';')
            .next()
            .unwrap_or_default()
            .split('{')
            .next()
            .unwrap_or_default();
        for candidate in rust_module_candidates(path) {
            if file_set.contains(&candidate) && candidate != relative_path {
                dependencies.insert(candidate);
            }
        }
    } else if trimmed.starts_with("use super::") || trimmed.starts_with("pub use super::") {
        let path = trimmed
            .split("super::")
            .nth(1)
            .unwrap_or_default()
            .split(';')
            .next()
            .unwrap_or_default()
            .split('{')
            .next()
            .unwrap_or_default();
        for candidate in relative_rust_candidates(relative_path, path, true) {
            if file_set.contains(&candidate) && candidate != relative_path {
                dependencies.insert(candidate);
            }
        }
    } else if trimmed.starts_with("mod ") || trimmed.starts_with("pub mod ") {
        let module = trimmed
            .trim_start_matches("pub ")
            .trim_start_matches("mod ")
            .trim_end_matches(';')
            .trim();
        for candidate in relative_rust_candidates(relative_path, module, false) {
            if file_set.contains(&candidate) && candidate != relative_path {
                dependencies.insert(candidate);
            }
        }
    }
}

fn parse_script_dependency(
    root: &Path,
    relative_path: &str,
    line: &str,
    file_set: &HashSet<String>,
    dependencies: &mut HashSet<String>,
) {
    let trimmed = line.trim();
    let Some(import_path) = extract_quoted_dependency(trimmed) else {
        return;
    };

    if !import_path.starts_with('.') {
        return;
    }

    for candidate in resolve_relative_script_import(root, relative_path, &import_path) {
        if file_set.contains(&candidate) && candidate != relative_path {
            dependencies.insert(candidate);
        }
    }
}

fn parse_python_dependency(
    relative_path: &str,
    line: &str,
    file_set: &HashSet<String>,
    dependencies: &mut HashSet<String>,
) {
    let trimmed = line.trim();
    if let Some(module) = trimmed.strip_prefix("from ") {
        let module = module.split_whitespace().next().unwrap_or_default();
        for candidate in resolve_python_module(relative_path, module) {
            if file_set.contains(&candidate) && candidate != relative_path {
                dependencies.insert(candidate);
            }
        }
    } else if let Some(module) = trimmed.strip_prefix("import ") {
        let module = module.split_whitespace().next().unwrap_or_default();
        for candidate in resolve_python_module(relative_path, module) {
            if file_set.contains(&candidate) && candidate != relative_path {
                dependencies.insert(candidate);
            }
        }
    }
}

fn reverse_graph(graph: &HashMap<String, BTreeSet<String>>) -> HashMap<String, BTreeSet<String>> {
    let mut reverse = HashMap::new();

    for (file, dependencies) in graph {
        reverse.entry(file.clone()).or_insert_with(BTreeSet::new);
        for dependency in dependencies {
            reverse
                .entry(dependency.clone())
                .or_insert_with(BTreeSet::new)
                .insert(file.clone());
        }
    }

    reverse
}

fn collect_dependents(
    changed: &HashSet<String>,
    reverse: &HashMap<String, BTreeSet<String>>,
    max_depth: usize,
) -> BTreeSet<String> {
    let mut queue = VecDeque::new();
    let mut seen = changed.clone();
    let mut dependents = BTreeSet::new();

    for file in changed {
        queue.push_back((file.clone(), 0usize));
    }

    while let Some((file, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }

        if let Some(parents) = reverse.get(&file) {
            for parent in parents {
                if seen.insert(parent.clone()) {
                    dependents.insert(parent.clone());
                    queue.push_back((parent.clone(), depth + 1));
                }
            }
        }
    }

    dependents
}

fn detect_cycles(
    graph: &HashMap<String, BTreeSet<String>>,
    relevant: &HashSet<String>,
    max_depth: usize,
) -> Vec<Vec<String>> {
    let mut cycles = BTreeSet::new();

    for start in relevant {
        let mut path = vec![start.clone()];
        let mut visited = HashSet::from([start.clone()]);
        dfs_cycles(
            start,
            start,
            graph,
            relevant,
            max_depth,
            &mut path,
            &mut visited,
            &mut cycles,
        );
    }

    cycles
        .into_iter()
        .map(|cycle| cycle.split('\u{1f}').map(|item| item.to_string()).collect())
        .collect()
}

fn dfs_cycles(
    start: &str,
    current: &str,
    graph: &HashMap<String, BTreeSet<String>>,
    relevant: &HashSet<String>,
    max_depth: usize,
    path: &mut Vec<String>,
    visited: &mut HashSet<String>,
    cycles: &mut BTreeSet<String>,
) {
    if path.len() > max_depth {
        return;
    }

    let Some(neighbors) = graph.get(current) else {
        return;
    };

    for neighbor in neighbors {
        if !relevant.contains(neighbor) {
            continue;
        }

        if neighbor == start && path.len() > 1 {
            let canonical = canonical_cycle(path);
            cycles.insert(canonical.join("\u{1f}"));
            continue;
        }

        if visited.insert(neighbor.clone()) {
            path.push(neighbor.clone());
            dfs_cycles(
                start, neighbor, graph, relevant, max_depth, path, visited, cycles,
            );
            path.pop();
            visited.remove(neighbor);
        }
    }
}

fn canonical_cycle(path: &[String]) -> Vec<String> {
    if path.is_empty() {
        return Vec::new();
    }

    let mut best = path.to_vec();
    for rotation in 1..path.len() {
        let rotated = path[rotation..]
            .iter()
            .chain(path[..rotation].iter())
            .cloned()
            .collect::<Vec<_>>();
        if rotated < best {
            best = rotated;
        }
    }
    best
}

fn score_impact(direct: usize, transitive: usize, cycles: usize, diff: &DiffSummary) -> u32 {
    let score = (direct as u32 * 24)
        + (transitive as u32 * 10)
        + (cycles as u32 * 12)
        + (diff.files_deleted as u32 * 10)
        + (diff.files_renamed as u32 * 8)
        + (diff.import_changes.len() as u32 * 3)
        + (diff.complexity_delta.unsigned_abs() as u32 * 2);

    score.min(100)
}

fn rust_module_candidates(module_path: &str) -> Vec<String> {
    let cleaned = module_path
        .split("::")
        .take_while(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if cleaned.is_empty() {
        return Vec::new();
    }

    let joined = cleaned.join("/");
    vec![
        format!("src/{joined}.rs"),
        format!("src/{joined}/mod.rs"),
        format!("src/{}.rs", cleaned[0]),
        format!("src/{}/mod.rs", cleaned[0]),
    ]
}

fn relative_rust_candidates(current_file: &str, module: &str, use_parent: bool) -> Vec<String> {
    let current_dir = Path::new(current_file)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let base_dir = if use_parent {
        current_dir.parent().unwrap_or(current_dir)
    } else {
        current_dir
    };
    let joined = module.replace("::", "/");
    vec![
        normalize_path(&base_dir.join(format!("{joined}.rs")).to_string_lossy()),
        normalize_path(&base_dir.join(format!("{joined}/mod.rs")).to_string_lossy()),
    ]
}

fn resolve_relative_script_import(
    root: &Path,
    current_file: &str,
    import_path: &str,
) -> Vec<String> {
    let current_dir = root.join(
        Path::new(current_file)
            .parent()
            .unwrap_or_else(|| Path::new("")),
    );
    let target = current_dir.join(import_path);
    let mut candidates = Vec::new();

    let base = normalize_path(&path_to_repo_relative(root, &target));
    candidates.push(base.clone());
    for extension in ["js", "ts", "tsx", "jsx", "mjs", "cjs"] {
        candidates.push(format!("{base}.{extension}"));
        candidates.push(format!("{base}/index.{extension}"));
    }

    candidates
}

fn resolve_python_module(current_file: &str, module: &str) -> Vec<String> {
    if module.is_empty() {
        return Vec::new();
    }

    let current_dir = Path::new(current_file)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    if let Some(relative) = module.strip_prefix('.') {
        let base_dir = current_dir.parent().unwrap_or(current_dir);
        let relative = relative.replace('.', "/");
        vec![
            normalize_path(&base_dir.join(format!("{relative}.py")).to_string_lossy()),
            normalize_path(
                &base_dir
                    .join(format!("{relative}/__init__.py"))
                    .to_string_lossy(),
            ),
        ]
    } else {
        let relative = module.replace('.', "/");
        vec![
            format!("{relative}.py"),
            format!("{relative}/__init__.py"),
            format!("src/{relative}.py"),
            format!("src/{relative}/__init__.py"),
        ]
    }
}

fn extract_quoted_dependency(line: &str) -> Option<String> {
    let quote = if line.contains('"') {
        '"'
    } else if line.contains('\'') {
        '\''
    } else {
        return None;
    };
    let start = line.find(quote)? + 1;
    let end = line[start..].find(quote)? + start;
    Some(line[start..end].to_string())
}

fn path_to_repo_relative(root: &Path, absolute: &PathBuf) -> String {
    absolute
        .strip_prefix(root)
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| absolute.to_string_lossy().to_string())
}

fn file_extension(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or_default()
}

fn is_supported_source_file(path: &str) -> bool {
    matches!(
        file_extension(path),
        "rs" | "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "py"
    )
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{build_dependency_graph, collect_dependents, reverse_graph};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn computes_direct_and_transitive_impacts_for_rust_modules() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("gitwhisper-impact-{unique}"));
        fs::create_dir_all(root.join("src")).expect("create src");
        fs::write(root.join("src/auth.rs"), "pub fn validate() {}\n").expect("write auth");
        fs::write(root.join("src/service.rs"), "use crate::auth::validate;\n")
            .expect("write service");
        fs::write(root.join("src/api.rs"), "use crate::service;\n").expect("write api");

        let graph = build_dependency_graph(&root);
        let reverse = reverse_graph(&graph);
        let changed = std::collections::HashSet::from(["src/auth.rs".to_string()]);
        let direct = collect_dependents(&changed, &reverse, 1);
        let transitive = collect_dependents(&changed, &reverse, 3);
        let _ = fs::remove_dir_all(&root);

        assert!(direct.contains(&"src/service.rs".to_string()));
        assert!(transitive.contains(&"src/api.rs".to_string()));
    }
}
