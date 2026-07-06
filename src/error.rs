use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("{0}")]
    Message(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Config parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Config serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Database error: {0}")]
    Postgres(#[from] postgres::Error),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Not in a git repository. Run from inside a git repo.")]
    NotGitRepo,

    #[error("GEMINI_API_KEY not set. Add it to .env or export it.")]
    MissingApiKey,

    #[error("AI request timed out after {seconds}s. Try increasing request_timeout_secs in .gitwhisper.toml")]
    Timeout { seconds: u64 },

    #[error("File not tracked by git: {path}")]
    FileNotTracked { path: String },
}

impl AppError {
    pub fn message(msg: impl Into<String>) -> Self {
        Self::Message(msg.into())
    }
}
