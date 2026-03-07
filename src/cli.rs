use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name="gitWhisper")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands
}

#[derive(Subcommand)]
pub enum Commands {
    Init,
    Capture,
    Log,
    Replay { commit: String },
    Explain {
        file: String,
        #[arg(short, long, default_value = "")]
        api_key: String,
    },
}