use std::fs;

pub fn install_hook() {
    let hook_script = r#"#!/bin/sh
gitWhisper capture
"#;

    fs::write(".git/hooks/post-commit", hook_script)
        .expect("Failed to write hook");

    println!("gitWhisper post-commit hook installed.");
}