use crate::git;

pub fn capture_context() {

    let commit = git::short_commit_hash();

    println!("Current commit: {}", commit);
}