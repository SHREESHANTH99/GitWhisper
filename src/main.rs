mod git;

fn main() {

    let hash = git::short_commit_hash();

    println!("Commit: {}", hash);
}