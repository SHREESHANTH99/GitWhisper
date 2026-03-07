use git2::Repository;

pub fn open_repo() -> Repository {
    Repository::discover(".")
        .expect("Not inside a git repository")
}

pub fn current_commit_hash() -> Option<String> {
    let repo = Repository::discover(".").ok()?;
    let head = repo.head().ok()?;
    let oid = head.target()?;

    Some(oid.to_string()) // convert git2::Oid to String
}

pub fn short_commit_hash() -> Option<String> {
    let full = current_commit_hash()?; // returns Option<String>

    Some(full[..7].to_string()) // take first 7 chars and convert to owned String
}