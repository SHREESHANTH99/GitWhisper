use crate::collaboration::CommitReport;
use crate::config::GithubConfig;
use crate::error::{AppError, AppResult};
use crate::integrations::RemoteRepository;
use reqwest::blocking::Client;
use serde_json::json;
use std::time::Duration;

pub fn publish_review(
    config: &GithubConfig,
    remote: &RemoteRepository,
    report: &CommitReport,
) -> AppResult<()> {
    ensure_configured(config)?;
    let client = client(config)?;
    let pull = find_open_pull_request(&client, config, remote, &report.branch)?;
    let body = render_review_body(report);

    let comment_url = format!(
        "{}/repos/{}/{}/issues/{}/comments",
        config.api_url.trim_end_matches('/'),
        remote.owner,
        remote.repo,
        pull.number
    );

    let response = client
        .post(comment_url)
        .json(&json!({ "body": body }))
        .send()?;
    if !response.status().is_success() {
        return Err(AppError::message(format!(
            "GitHub PR comment failed with status {}.",
            response.status()
        )));
    }

    if config.update_pr_description {
        let update_url = format!(
            "{}/repos/{}/{}/pulls/{}",
            config.api_url.trim_end_matches('/'),
            remote.owner,
            remote.repo,
            pull.number
        );
        let merged = append_section(
            pull.body.unwrap_or_default().as_str(),
            "Gitwhisper Review",
            &body,
        );
        let response = client
            .patch(update_url)
            .json(&json!({ "body": merged }))
            .send()?;
        if !response.status().is_success() {
            return Err(AppError::message(format!(
                "GitHub PR description update failed with status {}.",
                response.status()
            )));
        }
    }

    Ok(())
}

#[derive(Debug, serde::Deserialize)]
struct GithubPull {
    number: u64,
    #[serde(default)]
    body: Option<String>,
}

fn find_open_pull_request(
    client: &Client,
    config: &GithubConfig,
    remote: &RemoteRepository,
    branch: &str,
) -> AppResult<GithubPull> {
    let head = format!("{}:{}", remote.owner, branch);
    let url = format!(
        "{}/repos/{}/{}/pulls?state=open&head={}",
        config.api_url.trim_end_matches('/'),
        remote.owner,
        remote.repo,
        head
    );
    let response = client.get(url).send()?;
    if !response.status().is_success() {
        return Err(AppError::message(format!(
            "GitHub PR lookup failed with status {}.",
            response.status()
        )));
    }

    let pulls = response.json::<Vec<GithubPull>>().unwrap_or_default();
    pulls
        .into_iter()
        .next()
        .ok_or_else(|| AppError::message(format!("No open GitHub PR found for branch `{branch}`.")))
}

fn render_review_body(report: &CommitReport) -> String {
    let mut body = String::new();
    body.push_str("## Gitwhisper Review Summary\n\n");
    body.push_str(&format!("**Commit:** `{}`\n\n", report.commit));
    body.push_str(&format!("**Summary:** {}\n\n", report.summary));
    body.push_str(&format!(
        "**Risk:** {}\n\n",
        report.risk.as_deref().unwrap_or("unknown")
    ));

    if let Some(impact) = &report.impact {
        body.push_str(&format!("**Impact:** {}\n\n", impact));
    }

    if !report.changed_files.is_empty() {
        body.push_str("**Files changed**\n");
        for file in report.changed_files.iter().take(10) {
            body.push_str(&format!("- `{}`\n", file));
        }
        body.push('\n');
    }

    if let Some(review) = &report.review_summary {
        body.push_str(&format!("**Review/Test context:** {}\n\n", review));
    }

    if !report.related_history.is_empty() {
        body.push_str("**Related history**\n");
        for entry in report.related_history.iter().take(3) {
            body.push_str(&format!(
                "- `{}` {} ({})\n",
                entry.short_hash, entry.subject, entry.file
            ));
        }
    }

    body
}

fn append_section(existing: &str, title: &str, section: &str) -> String {
    let marker = format!("## {title}");
    if existing.contains(&marker) {
        existing.to_string()
    } else if existing.trim().is_empty() {
        section.to_string()
    } else {
        format!("{}\n\n{}", existing.trim_end(), section)
    }
}

fn ensure_configured(config: &GithubConfig) -> AppResult<()> {
    if !config.enabled {
        return Err(AppError::message(
            "GitHub integration is disabled in `.gitwhisper.toml`.",
        ));
    }

    if config.token.trim().is_empty() {
        return Err(AppError::message(
            "GitHub integration requires `integrations.github.token`.",
        ));
    }

    Ok(())
}

fn client(config: &GithubConfig) -> AppResult<Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static("gitwhisper"),
    );
    headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("application/vnd.github+json"),
    );
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", config.token.trim()))
            .map_err(|_| AppError::message("Invalid GitHub token header value."))?,
    );

    Ok(Client::builder()
        .timeout(Duration::from_secs(20))
        .default_headers(headers)
        .build()?)
}
