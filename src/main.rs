mod capture;
mod cli;
mod git;
mod hooks;
mod storage;
mod viewer;
use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    // Optional: read API key from environment if not passed via CLI
    let default_api_key =
        std::env::var("GEMINI_API_KEY").unwrap_or_else(|_| String::from("AIzaSyAffeLbH0n0Vkz4DTLFHKTwSnts4kpQ3y4"));

    match cli.command {
        Commands::Init => hooks::install_hook(),
        Commands::Capture => capture::capture_context(),
        Commands::Log => storage::show_logs(),
        Commands::Replay { commit } => viewer::replay::replay_commit(&commit),
        Commands::Explain { file, api_key } => {
            let key_to_use = if api_key.is_empty() { &default_api_key } else { &api_key };
            viewer::explain::explain_file(&file, key_to_use);
        }
    }
}
