use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const CONFIG_FILE_NAME: &str = ".gitwhisper.toml";
const DEFAULT_AI_MODEL: &str = "gemini-1.5-flash";
const DEFAULT_LOCAL_MODEL: &str = "mistral";
const DEFAULT_HISTORY_DEPTH: usize = 10;
const DEFAULT_TIMEOUT_SECS: u64 = 45;
const DEFAULT_COMMAND_LIMIT: usize = 25;
const DEFAULT_PROMPT_CHAR_BUDGET: usize = 12_000;
const DEFAULT_HYBRID_MAX_PROMPT_CHARS: usize = 8_000;
const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
const DEFAULT_GIT_NOTES_REF: &str = "refs/notes/gitwhisper";
const DEFAULT_WEBHOOK_TIMEOUT_SECS: u64 = 10;
const DEFAULT_GITHUB_API_URL: &str = "https://api.github.com";
const DEFAULT_GITLAB_API_URL: &str = "https://gitlab.com/api/v4";
const DEFAULT_DB_PATH: &str = ".git/gitwhisper/gitwhisper.db";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub ai: AiConfig,
    pub capture: CaptureConfig,
    pub collaboration: CollaborationConfig,
    pub integrations: IntegrationsConfig,
    pub privacy: PrivacyConfig,
    pub database: DatabaseConfig,
    pub audit: AuditConfig,
    pub auth: AuthConfig,
    pub feedback: FeedbackConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    pub provider: AiProvider,
    pub model: String,
    pub local_model: String,
    pub prompt_char_budget: usize,
    pub hybrid_max_prompt_chars: usize,
    pub ollama_url: String,
    pub history_depth: usize,
    pub request_timeout_secs: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AiProvider {
    Cloud,
    Local,
    Hybrid,
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
pub struct CollaborationConfig {
    pub auto_annotate_commits: bool,
    pub enable_git_notes: bool,
    pub git_notes_ref: String,
    pub webhook_url: String,
    pub webhook_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IntegrationsConfig {
    pub slack: SlackConfig,
    pub discord: DiscordConfig,
    pub github: GithubConfig,
    pub gitlab: GitlabConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SlackConfig {
    pub enabled: bool,
    pub webhook_url: String,
    pub bot_token: String,
    pub channel: String,
    pub auto_share_on_commit: bool,
    pub include_digest: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DiscordConfig {
    pub enabled: bool,
    pub webhook_url: String,
    pub auto_share_on_commit: bool,
    pub include_digest: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GithubConfig {
    pub enabled: bool,
    pub token: String,
    pub api_url: String,
    pub auto_comment_on_pr: bool,
    pub update_pr_description: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GitlabConfig {
    pub enabled: bool,
    pub token: String,
    pub api_url: String,
    pub auto_comment_on_mr: bool,
    pub update_mr_description: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PrivacyConfig {
    pub offline_mode: bool,
    pub local_cache_only: bool,
    pub exclude_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub backend: DatabaseBackend,
    pub path: String,
    pub postgres_url: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DatabaseBackend {
    Json,
    Postgres,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuditConfig {
    pub enabled: bool,
    pub retain_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    pub enabled: bool,
    pub mode: AuthMode,
    pub default_role: UserRole,
    pub users: Vec<AuthUserConfig>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AuthMode {
    Disabled,
    Local,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum UserRole {
    Viewer,
    Contributor,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthUserConfig {
    pub username: String,
    pub role: UserRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FeedbackConfig {
    pub enabled: bool,
    pub allow_anonymous: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ai: AiConfig::default(),
            capture: CaptureConfig::default(),
            collaboration: CollaborationConfig::default(),
            integrations: IntegrationsConfig::default(),
            privacy: PrivacyConfig::default(),
            database: DatabaseConfig::default(),
            audit: AuditConfig::default(),
            auth: AuthConfig::default(),
            feedback: FeedbackConfig::default(),
        }
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: AiProvider::Cloud,
            model: DEFAULT_AI_MODEL.to_string(),
            local_model: DEFAULT_LOCAL_MODEL.to_string(),
            prompt_char_budget: DEFAULT_PROMPT_CHAR_BUDGET,
            hybrid_max_prompt_chars: DEFAULT_HYBRID_MAX_PROMPT_CHARS,
            ollama_url: DEFAULT_OLLAMA_URL.to_string(),
            history_depth: DEFAULT_HISTORY_DEPTH,
            request_timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for AiProvider {
    fn default() -> Self {
        Self::Cloud
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

impl Default for CollaborationConfig {
    fn default() -> Self {
        Self {
            auto_annotate_commits: true,
            enable_git_notes: true,
            git_notes_ref: DEFAULT_GIT_NOTES_REF.to_string(),
            webhook_url: String::new(),
            webhook_timeout_secs: DEFAULT_WEBHOOK_TIMEOUT_SECS,
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for IntegrationsConfig {
    fn default() -> Self {
        Self {
            slack: SlackConfig::default(),
            discord: DiscordConfig::default(),
            github: GithubConfig::default(),
            gitlab: GitlabConfig::default(),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for SlackConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            webhook_url: String::new(),
            bot_token: String::new(),
            channel: String::new(),
            auto_share_on_commit: false,
            include_digest: false,
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            webhook_url: String::new(),
            auto_share_on_commit: false,
            include_digest: false,
        }
    }
}

impl Default for GithubConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token: String::new(),
            api_url: DEFAULT_GITHUB_API_URL.to_string(),
            auto_comment_on_pr: false,
            update_pr_description: false,
        }
    }
}

impl Default for GitlabConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token: String::new(),
            api_url: DEFAULT_GITLAB_API_URL.to_string(),
            auto_comment_on_mr: false,
            update_mr_description: false,
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

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            backend: DatabaseBackend::Json,
            path: DEFAULT_DB_PATH.to_string(),
            postgres_url: String::new(),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for DatabaseBackend {
    fn default() -> Self {
        Self::Json
    }
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            retain_days: 90,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: AuthMode::Disabled,
            default_role: UserRole::Admin,
            users: Vec::new(),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for AuthMode {
    fn default() -> Self {
        Self::Disabled
    }
}

#[allow(clippy::derivable_impls)]
impl Default for UserRole {
    fn default() -> Self {
        Self::Viewer
    }
}

impl Default for AuthUserConfig {
    fn default() -> Self {
        Self {
            username: String::new(),
            role: UserRole::Viewer,
        }
    }
}

impl Default for FeedbackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allow_anonymous: false,
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
        let mut config = match fs::read_to_string(path) {
            Ok(raw) => toml::from_str::<AppConfig>(&raw)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(error) => return Err(error.into()),
        };
        config.apply_env_overrides()?;
        Ok(config)
    }

    fn apply_env_overrides(&mut self) -> AppResult<()> {
        if let Some(backend) =
            read_env_first(&["GITWHISPER_DATABASE_BACKEND", "GITWHISPER_DB_BACKEND"])
        {
            self.database.backend = parse_database_backend(&backend)?;
        }

        if let Some(path) = read_env_first(&["GITWHISPER_DATABASE_PATH", "GITWHISPER_DB_PATH"]) {
            self.database.path = path;
        }

        if let Some(url) = read_env_first(&["GITWHISPER_POSTGRES_URL", "GITWHISPER_DATABASE_URL"]) {
            self.database.postgres_url = url;
        }

        Ok(())
    }
}

fn read_env_first(keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| std::env::var(key).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_database_backend(raw: &str) -> AppResult<DatabaseBackend> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "json" => Ok(DatabaseBackend::Json),
        "postgres" | "postgresql" => Ok(DatabaseBackend::Postgres),
        other => Err(AppError::message(format!(
            "Unsupported database backend `{other}`. Use `json` or `postgres`."
        ))),
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
        assert_eq!(config.ai.local_model, "mistral");
        assert_eq!(config.ai.provider, super::AiProvider::Cloud);
        assert_eq!(config.ai.history_depth, 10);
        assert_eq!(config.ai.prompt_char_budget, 12_000);
        assert_eq!(config.capture.command_limit, 25);
        assert!(config.collaboration.auto_annotate_commits);
        assert_eq!(config.collaboration.git_notes_ref, "refs/notes/gitwhisper");
        assert_eq!(config.integrations.github.api_url, "https://api.github.com");
        assert_eq!(
            config.integrations.gitlab.api_url,
            "https://gitlab.com/api/v4"
        );
        assert!(config.capture.include_analysis);
        assert!(config.privacy.local_cache_only);
        assert_eq!(config.database.backend, super::DatabaseBackend::Json);
        assert!(config.audit.enabled);
        assert_eq!(config.auth.mode, super::AuthMode::Disabled);
        assert!(config.feedback.enabled);
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
        assert_eq!(config.ai.prompt_char_budget, 12_000);
        assert!(config.collaboration.enable_git_notes);
        assert!(!config.integrations.slack.enabled);
        assert!(config.privacy.offline_mode);
        assert_eq!(config.capture.command_limit, 25);
        assert_eq!(config.database.path, ".git/gitwhisper/gitwhisper.db");
        assert!(config.database.postgres_url.is_empty());
    }

    #[test]
    fn parses_database_backend_names() {
        assert_eq!(
            super::parse_database_backend("json").expect("json should parse"),
            super::DatabaseBackend::Json
        );
        assert_eq!(
            super::parse_database_backend("postgresql").expect("postgresql should parse"),
            super::DatabaseBackend::Postgres
        );
        assert!(super::parse_database_backend("sqlite").is_err());
    }
}
