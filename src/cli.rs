use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name="git-lens")]
pub struct Cli {

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    Log,
    Replay { commit: String },
}

pub fn run() {

    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            println!("CommitLens initialized");
        }

        Commands::Log => {
            println!("Showing commit logs");
        }

        Commands::Replay { commit } => {
            println!("Replaying commit {}", commit);
        }
    }
}