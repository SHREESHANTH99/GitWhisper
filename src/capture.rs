use crate::git;
use crate::storage;

pub fn capture_context() {

    if let Some(commit) = git::short_commit_hash() {

        println!("Capturing context for commit {}", commit);

        storage::save_context(&commit);

    } else {

        println!("No commit found yet.");
    }
}