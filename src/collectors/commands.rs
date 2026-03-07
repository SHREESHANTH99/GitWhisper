use std::fs;
use dirs::home_dir;

pub fn get_recent_commands() -> Vec<String> {

    let home = home_dir().unwrap();
    let path = home.join(".bash_history");

    let content = fs::read_to_string(path).unwrap_or_default();

    content
        .lines()
        .rev()
        .take(10)
        .map(|s| s.to_string())
        .collect()
}