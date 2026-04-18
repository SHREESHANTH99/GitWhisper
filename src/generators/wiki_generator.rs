use crate::error::AppResult;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn generate_wiki(output_dir: &str) {
    match generate_wiki_inner(output_dir) {
        Ok(path) => println!("Generated wiki at {}", path.display()),
        Err(error) => eprintln!("{error}"),
    }
}

fn generate_wiki_inner(output_dir: &str) -> AppResult<PathBuf> {
    let snapshot = crate::metrics::collect_snapshot()?;
    let contexts = crate::storage::load::load_all_contexts()?;
    let output = PathBuf::from(output_dir);
    let files_dir = output.join("files");
    let people_dir = output.join("people");

    fs::create_dir_all(&files_dir)?;
    fs::create_dir_all(&people_dir)?;

    let mut search_index = Vec::new();

    let mut index = String::new();
    index.push_str("# Gitwhisper Wiki\n\n");
    index.push_str(&format!(
        "Generated: {}\n\n",
        snapshot.generated_at
    ));
    index.push_str("## Overview\n\n");
    index.push_str(&format!(
        "- Commits captured: {}\n- Contributors: {}\n- Files touched: {}\n\n",
        snapshot.overview.total_commits,
        snapshot.overview.unique_authors,
        snapshot.overview.files_touched
    ));

    index.push_str("## Hot Files\n\n");
    for file in snapshot.files.iter().take(15) {
        let file_name = slugify(&file.path);
        index.push_str(&format!(
            "- [{}](files/{}.md): {} commits, top owner {} ({:.0}%)\n",
            file.path,
            file_name,
            file.commits,
            file.top_author,
            file.top_author_share * 100.0
        ));
        search_index.push(serde_json::json!({
            "title": file.path,
            "path": format!("files/{}.md", file_name),
            "keywords": [file.path, file.top_author],
        }));
        write_file_page(&files_dir, file.path.as_str())?;
    }

    index.push_str("\n## People\n\n");
    for person in snapshot.people.iter().take(10) {
        let person_name = slugify(&person.author);
        index.push_str(&format!(
            "- [{}](people/{}.md): {} commits across {} files\n",
            person.author, person_name, person.commits, person.files_touched
        ));
        search_index.push(serde_json::json!({
            "title": person.author,
            "path": format!("people/{}.md", person_name),
            "keywords": person.top_files,
        }));
        write_person_page(&people_dir, person.author.as_str(), &contexts)?;
    }

    if !snapshot.risks.is_empty() {
        index.push_str("\n## Risks\n\n");
        for risk in snapshot.risks.iter().take(10) {
            index.push_str(&format!(
                "- **{}** {}: {}\n",
                risk.kind, risk.subject, risk.detail
            ));
        }
    }

    fs::write(output.join("index.md"), index)?;
    fs::write(
        output.join("search-index.json"),
        serde_json::to_string_pretty(&search_index)?,
    )?;

    Ok(output)
}

fn write_file_page(files_dir: &Path, file: &str) -> AppResult<()> {
    let history = crate::git::file_history(file, 12).unwrap_or_default();
    let owners = crate::git::owners_for_path(file, 5).unwrap_or_default();
    let file_name = slugify(file);
    let mut page = String::new();
    page.push_str(&format!("# {}\n\n", file));

    if !owners.is_empty() {
        page.push_str("## Owners\n\n");
        for owner in owners {
            page.push_str(&format!(
                "- {} <{}>: {} commits\n",
                owner.name, owner.email, owner.commits
            ));
        }
        page.push('\n');
    }

    page.push_str("## Timeline\n\n");
    if history.is_empty() {
        page.push_str("No Git history found.\n");
    } else {
        for entry in history {
            page.push_str(&format!(
                "- `{}` {} ({})\n",
                entry.short_hash, entry.subject, entry.timestamp
            ));
        }
    }

    fs::write(files_dir.join(format!("{file_name}.md")), page)?;
    Ok(())
}

fn write_person_page(people_dir: &Path, author: &str, contexts: &[crate::storage::context::CommitContext]) -> AppResult<()> {
    let mut per_file: HashMap<String, usize> = HashMap::new();
    let mut commits = Vec::new();

    for context in contexts {
        let context_author = if !context.behavior.author.trim().is_empty() {
            context.behavior.author.as_str()
        } else {
            ""
        };

        if context_author == author {
            for file in &context.files {
                *per_file.entry(file.clone()).or_insert(0) += 1;
            }
            commits.push(context.commit.clone());
        }
    }

    let mut top_files = per_file.into_iter().collect::<Vec<_>>();
    top_files.sort_by(|left, right| right.1.cmp(&left.1));
    let mut page = String::new();
    page.push_str(&format!("# {}\n\n", author));
    page.push_str(&format!("- Commits captured: {}\n\n", commits.len()));
    page.push_str("## Top Files\n\n");
    for (file, count) in top_files.into_iter().take(15) {
        page.push_str(&format!("- {} ({})\n", file, count));
    }

    page.push_str("\n## Recent Commits\n\n");
    for commit in commits.into_iter().take(12) {
        let subject = crate::git::commit_subject(&commit).unwrap_or_default();
        page.push_str(&format!("- `{}` {}\n", commit, subject));
    }

    fs::write(people_dir.join(format!("{}.md", slugify(author))), page)?;
    Ok(())
}

fn slugify(input: &str) -> String {
    let mut output = String::new();
    let mut last_dash = false;

    for ch in input.chars() {
        let normalized = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '-'
        };

        if normalized == '-' {
            if !last_dash {
                output.push('-');
                last_dash = true;
            }
        } else {
            output.push(normalized);
            last_dash = false;
        }
    }

    output.trim_matches('-').to_string()
}

