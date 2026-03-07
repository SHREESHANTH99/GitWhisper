use std::fs;

pub fn install_hook() {

    let hook = r#"#!/bin/sh
commitlens capture
"#;

    fs::write(".git/hooks/post-commit", hook)
        .expect("Failed to write hook");

    println!("Post-commit hook installed.");
}