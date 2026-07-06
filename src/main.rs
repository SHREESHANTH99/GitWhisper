mod ai;
mod analysis;
mod audit;
mod auth;
mod capture;
mod cli;
mod collaboration;
mod collectors;
mod config;
mod dashboard;
mod db;
mod error;
mod feedback;
mod generators;
mod git;
mod history;
mod hooks;
mod integrations;
mod metrics;
mod spinner;
mod storage;
mod viewer;

use clap::Parser;
use cli::{Cli, Commands};

fn run_command(result: crate::error::AppResult<()>) {
    if let Err(error) = result {
        use colored::Colorize;
        eprintln!("{} {}", "❌ Error:".red().bold(), error);
        std::process::exit(1);
    }
}

fn requires_ai(command: &Commands) -> bool {
    matches!(
        command,
        Commands::Annotate { .. } | Commands::Explain { .. } | Commands::Summarize { .. }
    )
}

fn main() {
    let cli = Cli::parse();

    dotenvy::dotenv().ok();

    let default_api_key = std::env::var("GEMINI_API_KEY").unwrap_or_else(|_| String::new());

    if requires_ai(&cli.command) && default_api_key.is_empty() {
        eprintln!("Note: GEMINI_API_KEY not set. Will use local Ollama or heuristics.");
    }

    match cli.command {
        Commands::Init => hooks::install_hook(),
        Commands::Capture => capture::capture_context(),
        Commands::Annotate { commit } => {
            collaboration::annotate_commit(commit.as_deref(), &default_api_key);
        }
        Commands::Share { provider, commit } => {
            let provider = match provider {
                cli::ShareProvider::Slack => "slack",
                cli::ShareProvider::Discord => "discord",
            };
            integrations::share_commit(provider, commit.as_deref(), &default_api_key);
        }
        Commands::Review { provider, commit } => {
            let provider = match provider {
                cli::ReviewProvider::Github => "github",
                cli::ReviewProvider::Gitlab => "gitlab",
            };
            integrations::publish_review(provider, commit.as_deref(), &default_api_key);
        }
        Commands::Digest { provider, period } => {
            let provider = match provider {
                cli::ShareProvider::Slack => "slack",
                cli::ShareProvider::Discord => "discord",
            };
            let period = match period {
                cli::DigestPeriod::Daily => "daily",
                cli::DigestPeriod::Weekly => "weekly",
            };
            integrations::send_digest(provider, period);
        }
        Commands::Log => viewer::log::show_logs(),
        Commands::Replay { commit } => viewer::replay::replay_commit(commit.as_deref()),
        Commands::Timeline { file } => viewer::timeline::show_timeline(&file),
        Commands::Explain { file } => {
            viewer::explain::explain_file(&file, &default_api_key);
        }
        Commands::Summarize { file } => {
            viewer::summarize::summarize_file(&file, &default_api_key);
        }
        Commands::Quality { path } => run_command(viewer::quality::show_quality(&path)),
        Commands::Security { path } => viewer::security::show_security(&path),
        Commands::Performance { path } => viewer::performance::show_performance(&path),
        Commands::BugPredict { path, limit } => {
            viewer::bug_predict::show_bug_predictions(path.as_deref(), limit)
        }
        Commands::KnowledgeRisk { path, limit } => {
            viewer::knowledge_risk::show_knowledge_risk(path.as_deref(), limit)
        }
        Commands::RefactorPriority { path, limit } => {
            viewer::refactor_priority::show_refactor_priority(path.as_deref(), limit)
        }
        Commands::Feedback {
            commit,
            good,
            poor,
            correct,
            tags,
        } => feedback::show_feedback(&commit, good, poor, &correct, &tags),
        Commands::FeedbackLog { limit } => feedback::show_recent_feedback(limit),
        Commands::FeedbackExport { format, output } => {
            let format = match format {
                cli::ExportFormat::Json => "json",
                cli::ExportFormat::Csv => "csv",
            };
            feedback::show_feedback_export(&output, format);
        }
        Commands::WhoAmI => viewer::auth::show_current_user(),
        Commands::AuditLog { limit } => viewer::audit_log::show_audit_log(limit),
        Commands::AuditPrune { days } => viewer::audit_log::prune_audit_log(days),
        Commands::Owners { path, limit } => viewer::owners::show_owners(&path, limit),
        Commands::Dashboard { host, port } => dashboard::serve_dashboard(&host, port),
        Commands::Export { format, output } => {
            let snapshot = match metrics::collect_snapshot() {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    eprintln!("{error}");
                    return;
                }
            };
            let format = match format {
                cli::ExportFormat::Json => "json",
                cli::ExportFormat::Csv => "csv",
            };
            if let Err(error) =
                metrics::exporter::export_snapshot(&snapshot, format, std::path::Path::new(&output))
            {
                eprintln!("{error}");
            } else {
                println!("Exported analytics snapshot to {}", output);
            }
        }
        Commands::Wiki { output } => generators::wiki_generator::generate_wiki(&output),
        Commands::Adr { output } => generators::adr_generator::generate_adrs(&output),
        Commands::PostCommit => collaboration::run_post_commit(&default_api_key),
        Commands::Search { query, limit, json } => run_command(viewer::search::show_search(
            &query,
            limit,
            json,
            &default_api_key,
        )),
    }
}
