mod ai;
mod analysis;
mod capture;
mod cli;
mod collectors;
mod config;
mod error;
mod git;
mod history;
mod hooks;
mod storage;
mod viewer;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    dotenvy::dotenv().ok();

    let default_api_key = std::env::var("GEMINI_API_KEY").unwrap_or_else(|_| String::new());
    match cli.command {
        Commands::Init => hooks::install_hook(),
        Commands::Capture => capture::capture_context(),
        Commands::Log => viewer::log::show_logs(),
        Commands::Replay { commit } => viewer::replay::replay_commit(commit.as_deref()),
        Commands::Timeline { file } => viewer::timeline::show_timeline(&file),
        Commands::Explain { file, api_key } => {
            let key_to_use = if api_key.is_empty() {
                &default_api_key
            } else {
                &api_key
            };
            viewer::explain::explain_file(&file, key_to_use);
        }
        Commands::Summarize { file, api_key } => {
            let key_to_use = if api_key.is_empty() {
                &default_api_key
            } else {
                &api_key
            };
            viewer::summarize::summarize_file(&file, key_to_use);
        }
        Commands::Owners { path, limit } => viewer::owners::show_owners(&path, limit),
    }
}
