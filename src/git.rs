use git2::Repository;

// Return the full commit hash
pub fn current_commit_hash() -> Option<String> {
    let repo = Repository::discover(".").ok()?;
    let head = repo.head().ok()?;
    let oid = head.target()?;
    Some(oid.to_string())
}

// Return first 7 chars (like git)
pub fn short_commit_hash() -> Option<String> {
    let full = current_commit_hash()?;
    Some(full[..7].to_string())
}