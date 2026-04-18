use crate::collaboration::CommitReport;
use crate::config::DiscordConfig;
use crate::error::{AppError, AppResult};
use reqwest::blocking::Client;
use serde_json::json;
use std::time::Duration;

pub fn send_commit(
    config: &DiscordConfig,
    report: &CommitReport,
    commit_url: Option<&str>,
) -> AppResult<()> {
    ensure_configured(config)?;
    let mut embed = json!({
        "title": format!("Gitwhisper {}", report.commit),
        "description": format!("**{}**\n\n{}", report.subject, report.summary),
        "color": 16742912,
        "fields": [
            { "name": "Branch", "value": report.branch, "inline": true },
            { "name": "Risk", "value": report.risk.clone().unwrap_or_else(|| "unknown".to_string()), "inline": true },
            { "name": "Files", "value": truncate(report.changed_files.join(", ").as_str(), 900), "inline": false }
        ]
    });

    if let Some(url) = commit_url {
        embed["url"] = json!(url);
    }

    let response = client()?
        .post(format!("{}?wait=true", config.webhook_url.trim()))
        .json(&json!({
            "content": format!("Commit explained: {}", report.commit),
            "embeds": [embed]
        }))
        .send()?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(AppError::message(format!(
            "Discord webhook failed with status {}.",
            response.status()
        )))
    }
}

pub fn send_digest(config: &DiscordConfig, period: &str, digest: &str) -> AppResult<()> {
    ensure_configured(config)?;
    let response = client()?
        .post(format!("{}?wait=true", config.webhook_url.trim()))
        .json(&json!({
            "content": format!("Gitwhisper {} digest\n```text\n{}\n```", period, truncate(digest, 1800))
        }))
        .send()?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(AppError::message(format!(
            "Discord webhook failed with status {}.",
            response.status()
        )))
    }
}

fn ensure_configured(config: &DiscordConfig) -> AppResult<()> {
    if !config.enabled {
        return Err(AppError::message("Discord integration is disabled in `.gitwhisper.toml`."));
    }

    if config.webhook_url.trim().is_empty() {
        return Err(AppError::message("Discord requires `integrations.discord.webhook_url`."));
    }

    Ok(())
}

fn client() -> AppResult<Client> {
    Ok(Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?)
}

fn truncate(input: &str, max_len: usize) -> String {
    if input.chars().count() <= max_len {
        input.to_string()
    } else if max_len <= 3 {
        ".".repeat(max_len)
    } else {
        format!("{}...", input.chars().take(max_len - 3).collect::<String>())
    }
}

