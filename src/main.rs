mod git;
mod capture;
mod hooks;
mod storage;
mod cli;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {

    let cli = Cli::parse();

    match cli.command {

        Commands::Init => {
            hooks::install_hook();
        }

        Commands::Capture => {
            capture::capture_context();
        }

        Commands::Log => {
            storage::show_logs();
        }
    }
}