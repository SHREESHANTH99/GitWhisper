use crate::collaboration::CommitReport;
use crate::config::SlackConfig;
use crate::error::{AppError, AppResult};
use reqwest::blocking::Client;
use serde_json::json;
use std::time::Duration;

pub fn send_commit(
    config: &SlackConfig,
    report: &CommitReport,
    commit_url: Option<&str>,
) -> AppResult<()> {
    ensure_configured(config)?;

    let text = format!("Commit explained: {} | {}", report.commit, report.summary);
    let mut blocks = vec![
        json!({
            "type": "header",
            "text": {
                "type": "plain_text",
                "text": format!("Gitwhisper: {}", report.commit)
            }
        }),
        json!({
            "type": "section",
            "fields": [
                { "type": "mrkdwn", "text": format!("*Branch*\n{}", report.branch) },
                { "type": "mrkdwn", "text": format!("*Source*\n{}", report.source_label()) }
            ]
        }),
        json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": format!("*{}*\n{}", escape_markdown(&report.subject), escape_markdown(&report.summary))
            }
        }),
        json!({
            "type": "section",
            "fields": [
                { "type": "mrkdwn", "text": format!("*Files*\n{}", escape_markdown(&report.changed_files.join(", "))) },
                { "type": "mrkdwn", "text": format!("*Risk*\n{}", escape_markdown(report.risk.as_deref().unwrap_or("unknown"))) }
            ]
        }),
    ];

    if let Some(url) = commit_url {
        blocks.push(json!({
            "type": "actions",
            "elements": [{
                "type": "button",
                "text": {"type": "plain_text", "text": "Open Commit"},
                "url": url
            }]
        }));
    }

    post_message(config, json!({ "text": text, "blocks": blocks }))?;
    Ok(())
}

pub fn send_digest(config: &SlackConfig, period: &str, digest: &str) -> AppResult<()> {
    ensure_configured(config)?;
    let payload = json!({
        "text": format!("Gitwhisper {} digest", period),
        "blocks": [
            {
                "type": "header",
                "text": {"type": "plain_text", "text": format!("Gitwhisper {} digest", period)}
            },
            {
                "type": "section",
                "text": {"type": "mrkdwn", "text": escape_markdown(digest)}
            }
        ]
    });
    post_message(config, payload)?;
    Ok(())
}

fn post_message(config: &SlackConfig, payload: serde_json::Value) -> AppResult<()> {
    let client = client()?;

    if !config.bot_token.trim().is_empty() && !config.channel.trim().is_empty() {
        let response = client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(config.bot_token.trim())
            .json(&json!({
                "channel": config.channel,
                "text": payload.get("text").cloned().unwrap_or_else(|| json!("Gitwhisper update")),
                "blocks": payload.get("blocks").cloned().unwrap_or_else(|| json!([])),
            }))
            .send()?;

        let value = response
            .json::<serde_json::Value>()
            .unwrap_or_else(|_| json!({}));
        if value.get("ok").and_then(|ok| ok.as_bool()) == Some(true) {
            Ok(())
        } else {
            Err(AppError::message(format!(
                "Slack API request failed: {}",
                value
                    .get("error")
                    .and_then(|error| error.as_str())
                    .unwrap_or("unknown error")
            )))
        }
    } else {
        let webhook = config.webhook_url.trim();
        let response = client.post(webhook).json(&payload).send()?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(AppError::message(format!(
                "Slack webhook failed with status {}.",
                response.status()
            )))
        }
    }
}

fn ensure_configured(config: &SlackConfig) -> AppResult<()> {
    if !config.enabled {
        return Err(AppError::message(
            "Slack integration is disabled in `.gitwhisper.toml`.",
        ));
    }

    if config.webhook_url.trim().is_empty()
        && (config.bot_token.trim().is_empty() || config.channel.trim().is_empty())
    {
        return Err(AppError::message(
            "Slack requires either `webhook_url` or both `bot_token` and `channel`.",
        ));
    }

    Ok(())
}

fn client() -> AppResult<Client> {
    Ok(Client::builder().timeout(Duration::from_secs(15)).build()?)
}

fn escape_markdown(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

trait SourceLabel {
    fn source_label(&self) -> String;
}

impl SourceLabel for CommitReport {
    fn source_label(&self) -> String {
        match &self.ai_model {
            Some(model) => format!("{} ({})", self.source, model),
            None => self.source.clone(),
        }
    }
}
