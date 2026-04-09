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
    pub ide: IdeContext,
    pub review: ReviewContext,
    pub behavior: BehaviorSnapshot,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct IdeContext {
    pub name: String,
    pub process: String,
    pub version: String,
    pub build_system: String,
    pub extensions: Vec<String>,
    pub active_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct ReviewContext {
    pub ci_provider: String,
    pub pr_number: String,
    pub reviewers: Vec<String>,
    pub labels: Vec<String>,
    pub milestone: String,
    pub test_status: String,
    pub tests_run: usize,
    pub tests_failed: usize,
    pub coverage_percent: Option<u8>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct BehaviorSnapshot {
    pub author: String,
    pub commits_last_7d: usize,
    pub commits_last_30d: usize,
    pub late_night_ratio: u8,
    pub typical_work_hours: String,
    pub burnout_risk: BurnoutRisk,
    pub expertise: Vec<FileExpertise>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct FileExpertise {
    pub path: String,
    pub commit_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum BurnoutRisk {
    #[default]
    Normal,
    Watch,
    Elevated,
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

impl IdeContext {
    pub fn is_empty(&self) -> bool {
        self.name.trim().is_empty()
            && self.process.trim().is_empty()
            && self.build_system.trim().is_empty()
            && self.extensions.is_empty()
            && self.active_files.is_empty()
    }

    pub fn summary(&self) -> Option<String> {
        if self.is_empty() {
            None
        } else {
            let mut parts = Vec::new();
            if !self.name.trim().is_empty() {
                parts.push(self.name.clone());
            }
            if !self.build_system.trim().is_empty() {
                parts.push(format!("build {}", self.build_system));
            }
            if !self.active_files.is_empty() {
                parts.push(format!("{} active files", self.active_files.len()));
            }
            Some(parts.join(" | "))
        }
    }
}

impl ReviewContext {
    pub fn is_empty(&self) -> bool {
        self.ci_provider.trim().is_empty()
            && self.pr_number.trim().is_empty()
            && self.reviewers.is_empty()
            && self.labels.is_empty()
            && self.test_status.trim().is_empty()
            && self.coverage_percent.is_none()
    }

    pub fn summary(&self) -> Option<String> {
        if self.is_empty() {
            None
        } else {
            let mut parts = Vec::new();
            if !self.ci_provider.trim().is_empty() {
                parts.push(self.ci_provider.clone());
            }
            if !self.pr_number.trim().is_empty() {
                parts.push(format!("PR {}", self.pr_number));
            }
            if !self.test_status.trim().is_empty() {
                parts.push(format!("tests {}", self.test_status));
            }
            if let Some(coverage) = self.coverage_percent {
                parts.push(format!("coverage {}%", coverage));
            }
            Some(parts.join(" | "))
        }
    }
}

impl BehaviorSnapshot {
    pub fn is_empty(&self) -> bool {
        self.author.trim().is_empty()
            && self.commits_last_7d == 0
            && self.commits_last_30d == 0
            && self.expertise.is_empty()
    }

    pub fn summary(&self) -> Option<String> {
        if self.is_empty() {
            None
        } else {
            Some(format!(
                "{} commits/7d, {} commits/30d, burnout {}",
                self.commits_last_7d, self.commits_last_30d, self.burnout_risk
            ))
        }
    }

    pub fn expertise_summary(&self, limit: usize) -> Option<String> {
        if self.expertise.is_empty() {
            None
        } else {
            Some(
                self.expertise
                    .iter()
                    .take(limit)
                    .map(|entry| format!("{} ({})", entry.path, entry.commit_count))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        }
    }
}

fn default_schema_version() -> u32 {
    3
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

impl std::fmt::Display for BurnoutRisk {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Normal => "normal",
            Self::Watch => "watch",
            Self::Elevated => "elevated",
        };

        write!(formatter, "{label}")
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

        assert_eq!(context.schema_version, 3);
        assert_eq!(context.environment.os, "windows");
        assert_eq!(context.environment.branch, "main");
        assert_eq!(context.environment.tools.node, "v22.14.0");
        assert!(context.ide.is_empty());
        assert!(context.review.is_empty());
        assert!(context.behavior.is_empty());
    }
}
