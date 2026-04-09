use crate::analysis::CommitAnalysis;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CommitContext {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub commit: String,
    pub timestamp: String,
    pub commands: Vec<String>,
    #[serde(deserialize_with = "deserialize_environment")]
    pub environment: EnvironmentContext,
    pub files: Vec<String>,
    pub analysis: CommitAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct EnvironmentContext {
    pub os: String,
    pub branch: String,
    pub shell: String,
    pub working_directory: String,
    pub tools: ToolingContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct ToolingContext {
    pub node: String,
    pub python: String,
    pub rust: String,
}

impl EnvironmentContext {
    pub fn is_empty(&self) -> bool {
        self.os.trim().is_empty()
            && self.branch.trim().is_empty()
            && self.shell.trim().is_empty()
            && self.working_directory.trim().is_empty()
            && self.tools.node.trim().is_empty()
            && self.tools.python.trim().is_empty()
            && self.tools.rust.trim().is_empty()
    }

    pub fn to_display_string(&self) -> String {
        self.to_lines().join("\n")
    }

    pub fn to_prompt_string(&self) -> String {
        self.to_lines().join(" | ")
    }

    fn to_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();

        push_if_present(&mut lines, "OS", &self.os);
        push_if_present(&mut lines, "Branch", &self.branch);
        push_if_present(&mut lines, "Shell", &self.shell);
        push_if_present(&mut lines, "Working Directory", &self.working_directory);
        push_if_present(&mut lines, "Node", &self.tools.node);
        push_if_present(&mut lines, "Python", &self.tools.python);
        push_if_present(&mut lines, "Rust", &self.tools.rust);

        lines
    }

    fn from_legacy_string(input: &str) -> Self {
        let mut environment = Self::default();

        for line in input.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };

            let normalized_key = key.trim().to_ascii_lowercase();
            let value = value.trim().to_string();

            match normalized_key.as_str() {
                "os" => environment.os = value,
                "branch" => environment.branch = value,
                "shell" => environment.shell = value,
                "working directory" => environment.working_directory = value,
                "node" => environment.tools.node = value,
                "python" => environment.tools.python = value,
                "rust" => environment.tools.rust = value,
                _ => {}
            }
        }

        environment
    }
}

fn default_schema_version() -> u32 {
    2
}

fn push_if_present(lines: &mut Vec<String>, label: &str, value: &str) {
    if !value.trim().is_empty() {
        lines.push(format!("{label}: {value}"));
    }
}

fn deserialize_environment<'de, D>(deserializer: D) -> Result<EnvironmentContext, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::Null => Ok(EnvironmentContext::default()),
        serde_json::Value::String(raw) => Ok(EnvironmentContext::from_legacy_string(&raw)),
        serde_json::Value::Object(_) => {
            serde_json::from_value(value).map_err(|error| D::Error::custom(error.to_string()))
        }
        _ => Err(D::Error::custom(
            "environment must be a string or object representation",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::CommitContext;

    #[test]
    fn loads_legacy_environment_strings() {
        let raw = r#"{
            "commit": "abc1234",
            "timestamp": "2026-03-07T12:14:26Z",
            "commands": ["cargo test"],
            "environment": "OS: windows\nBranch: main\nNode: v22.14.0",
            "files": ["src/main.rs"]
        }"#;

        let context: CommitContext =
            serde_json::from_str(raw).expect("legacy context should deserialize");

        assert_eq!(context.schema_version, 2);
        assert_eq!(context.environment.os, "windows");
        assert_eq!(context.environment.branch, "main");
        assert_eq!(context.environment.tools.node, "v22.14.0");
    }
}
