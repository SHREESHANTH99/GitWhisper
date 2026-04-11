use clap::{Parser, Subcommand};

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
    /// Install the post-commit hook that captures context after each commit.
    Init,
    /// Capture context for the current HEAD commit.
    Capture,
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
    /// Show likely code owners (top contributors) for a file or directory.
    Owners {
        path: String,
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
}
