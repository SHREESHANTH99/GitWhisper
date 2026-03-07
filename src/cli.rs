use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name="commitlens")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands
}

#[derive(Subcommand)]
pub enum Commands {
    Init,
    Capture,
    Log
}