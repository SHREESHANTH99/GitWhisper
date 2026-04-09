use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const CONFIG_FILE_NAME: &str = ".gitwhisper.toml";
const DEFAULT_AI_MODEL: &str = "gemini-1.5-flash";
const DEFAULT_HISTORY_DEPTH: usize = 10;
const DEFAULT_TIMEOUT_SECS: u64 = 45;
const DEFAULT_COMMAND_LIMIT: usize = 25;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub ai: AiConfig,
    pub capture: CaptureConfig,
    pub privacy: PrivacyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    pub model: String,
    pub history_depth: usize,
    pub request_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureConfig {
    pub command_limit: usize,
    pub include_environment: bool,
    pub include_analysis: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PrivacyConfig {
    pub offline_mode: bool,
    pub local_cache_only: bool,
    pub exclude_files: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ai: AiConfig::default(),
            capture: CaptureConfig::default(),
            privacy: PrivacyConfig::default(),
        }
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            model: DEFAULT_AI_MODEL.to_string(),
            history_depth: DEFAULT_HISTORY_DEPTH,
            request_timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            command_limit: DEFAULT_COMMAND_LIMIT,
            include_environment: true,
            include_analysis: true,
        }
    }
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            offline_mode: false,
            local_cache_only: true,
            exclude_files: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn load() -> AppResult<Self> {
        let root = crate::git::repo_root()?;
        Self::load_from_repo_root(&root)
    }

    pub fn load_from_repo_root(root: &Path) -> AppResult<Self> {
        let path = root.join(CONFIG_FILE_NAME);
        Self::load_from_path(&path)
    }

    pub fn load_from_path(path: &Path) -> AppResult<Self> {
        match fs::read_to_string(path) {
            Ok(raw) => Ok(toml::from_str::<AppConfig>(&raw)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn uses_defaults_when_config_is_missing() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("gitwhisper-missing-{unique}.toml"));
        let config = AppConfig::load_from_path(&path).expect("missing config should use defaults");

        assert_eq!(config.ai.model, "gemini-1.5-flash");
        assert_eq!(config.ai.history_depth, 10);
        assert_eq!(config.capture.command_limit, 25);
        assert!(config.capture.include_analysis);
        assert!(config.privacy.local_cache_only);
    }

    #[test]
    fn parses_partial_config_and_keeps_defaults() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("gitwhisper-config-{unique}.toml"));
        std::fs::write(
            &path,
            "[ai]\nmodel = \"mistral\"\n\n[privacy]\noffline_mode = true\n",
        )
        .expect("temporary config should be writable");

        let config = AppConfig::load_from_path(&path).expect("config should parse");
        let _ = std::fs::remove_file(&path);

        assert_eq!(config.ai.model, "mistral");
        assert_eq!(config.ai.history_depth, 10);
        assert!(config.privacy.offline_mode);
        assert_eq!(config.capture.command_limit, 25);
    }
}
