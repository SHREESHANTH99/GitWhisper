use std::fs;
use serde::{Serialize, Deserialize};
use chrono::Utc;

#[derive(Serialize, Deserialize)]
pub struct Context {

    pub commit: String,
    pub timestamp: String,
}

pub fn save_context(commit: &str) {

    let dir = ".git/commitlens";

    fs::create_dir_all(dir).unwrap();

    let context = Context {

        commit: commit.to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };

    let json = serde_json::to_string_pretty(&context).unwrap();

    let file = format!("{}/{}.json", dir, commit);

    fs::write(file, json).unwrap();
}

pub fn show_logs() {

    let dir = ".git/commitlens";

    if let Ok(entries) = fs::read_dir(dir) {

        for entry in entries {

            let file = entry.unwrap().path();

            let data = fs::read_to_string(file).unwrap();

            println!("{}", data);
        }
    }
}