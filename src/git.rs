use git2::Repository;

pub fn open_repo() -> Repository {
    Repository::discover(".")
        .expect("Not inside a git repository")
}

pub fn current_commit_hash() -> String {

    let repo = open_repo();

    let head = repo.head()
        .expect("Cannot find HEAD");

    let oid = head.target()
        .expect("HEAD has no target");

    oid.to_string()
}

pub fn short_commit_hash() -> String {

    let full = current_commit_hash();

    full[..7].to_string()
}