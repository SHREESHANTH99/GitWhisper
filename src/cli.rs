use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "gitwhisper",
    about = "AI-powered Git commit intelligence for developers"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install the post-commit hook that captures context and annotates commits.
    Init,
    /// Capture context for the current HEAD commit.
    Capture,
    /// Capture context, generate a commit explanation, and store it in Git notes.
    Annotate {
        commit: Option<String>,
        #[arg(short, long, default_value = "")]
        api_key: String,
    },
    /// Share a commit explanation to Slack or Discord.
    Share {
        provider: ShareProvider,
        commit: Option<String>,
        #[arg(short, long, default_value = "")]
        api_key: String,
    },
    /// Publish a Gitwhisper review summary to GitHub or GitLab.
    Review {
        provider: ReviewProvider,
        commit: Option<String>,
        #[arg(short, long, default_value = "")]
        api_key: String,
    },
    /// Send a daily or weekly digest to Slack or Discord.
    Digest {
        provider: ShareProvider,
        #[arg(long, default_value = "weekly")]
        period: DigestPeriod,
    },
    /// Show saved commit context entries.
    Log,
    /// Replay captured activity for a commit. Defaults to the latest saved commit.
    Replay { commit: Option<String> },
    /// Show the commit timeline for a file.
    Timeline { file: String },
    /// Explain why a file changed using Git history plus captured context.
    Explain {
        file: String,
        #[arg(short, long, default_value = "")]
        api_key: String,
    },
    /// Summarize how a file evolved over time (milestones + narrative).
    Summarize {
        file: String,
        #[arg(short, long, default_value = "")]
        api_key: String,
    },
    /// Analyze code quality risk for a file or directory.
    Quality {
        path: String,
    },
    /// Analyze security risk for a file or directory.
    Security {
        path: String,
    },
    /// Analyze performance risk for a file or directory.
    Performance {
        path: String,
    },
    /// Predict which files are most bug-prone.
    BugPredict {
        path: Option<String>,
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
    /// Report knowledge-silo and ownership concentration risk.
    KnowledgeRisk {
        path: Option<String>,
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
    /// Rank refactor priority across a file set.
    RefactorPriority {
        path: Option<String>,
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
    /// Store explanation feedback for a commit.
    Feedback {
        commit: String,
        #[arg(long)]
        good: bool,
        #[arg(long)]
        poor: bool,
        #[arg(long, default_value = "")]
        correct: String,
        #[arg(long, default_value = "")]
        tags: String,
    },
    /// Show recent explanation feedback entries.
    FeedbackLog {
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    /// Export stored feedback entries.
    FeedbackExport {
        #[arg(long, default_value = "json")]
        format: ExportFormat,
        #[arg(long, default_value = "exports/gitwhisper-feedback.json")]
        output: String,
    },
    #[command(name = "whoami")]
    /// Show the current authenticated user and role.
    WhoAmI,
    /// Show recent audit events.
    AuditLog {
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    /// Prune old audit events.
    AuditPrune {
        #[arg(long)]
        days: Option<u32>,
    },
    /// Show likely code owners (top contributors) for a file or directory.
    Owners {
        path: String,
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
    /// Start the lightweight team analytics dashboard.
    Dashboard {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 7878)]
        port: u16,
    },
    /// Export analytics snapshot to JSON or CSV.
    Export {
        #[arg(long, default_value = "json")]
        format: ExportFormat,
        #[arg(long, default_value = "exports/gitwhisper-snapshot.json")]
        output: String,
    },
    /// Generate markdown wiki pages from captured project knowledge.
    Wiki {
        #[arg(long, default_value = "wiki")]
        output: String,
    },
    /// Generate ADR markdown files from decision-worthy commits.
    Adr {
        #[arg(long, default_value = "docs/adrs")]
        output: String,
    },
    #[command(hide = true)]
    PostCommit,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum ShareProvider {
    Slack,
    Discord,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum ReviewProvider {
    Github,
    Gitlab,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum DigestPeriod {
    Daily,
    Weekly,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum ExportFormat {
    Json,
    Csv,
}
