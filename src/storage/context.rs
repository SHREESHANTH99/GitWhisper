use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommitContext {
    pub commit: String,
    pub timestamp: String,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub environment: String,
    #[serde(default)]
    pub files: Vec<String>,
}
