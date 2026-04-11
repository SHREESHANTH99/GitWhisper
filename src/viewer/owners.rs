pub fn show_owners(path: &str, limit: usize) {
    let normalized = match crate::git::normalize_repo_path(path) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    let owners = match crate::git::owners_for_path(&normalized, limit.max(1)) {
        Ok(owners) => owners,
        Err(error) => {
            eprintln!("Could not read ownership for `{}`: {}", normalized, error);
            return;
        }
    };

    if owners.is_empty() {
        println!("No Git ownership data found for {}.", normalized);
        return;
    }

    let total_commits: usize = owners.iter().map(|owner| owner.commits).sum();

    println!("Owners for {}:\n", normalized);
    for (idx, owner) in owners.iter().enumerate() {
        let percent = if total_commits == 0 {
            0.0
        } else {
            owner.commits as f64 / total_commits as f64 * 100.0
        };

        let identity = if owner.email.trim().is_empty() {
            owner.name.clone()
        } else {
            format!("{} <{}>", owner.name, owner.email)
        };

        println!(
            "{:>2}. {} - {} commits ({:.0}%)",
            idx + 1,
            identity,
            owner.commits,
            percent
        );
    }

    if total_commits >= 10 {
        if let Some(top) = owners.first() {
            let ratio = if total_commits == 0 {
                0.0
            } else {
                top.commits as f64 / total_commits as f64
            };

            if ratio >= 0.80 {
                println!(
                    "\nRisk: knowledge silo (top owner has {:.0}% of commits).",
                    ratio * 100.0
                );
            } else if ratio >= 0.60 {
                println!(
                    "\nWatch: ownership concentrated (top owner has {:.0}% of commits).",
                    ratio * 100.0
                );
            }
        }
    }
}

