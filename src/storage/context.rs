use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Context {

    pub commit: String,
    pub timestamp: String,
    pub commands: Vec<String>,
    pub node: Option<String>,
    pub os: String,
}