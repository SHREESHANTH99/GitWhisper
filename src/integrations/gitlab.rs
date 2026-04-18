use crate::collaboration::CommitReport;
use crate::config::GitlabConfig;
use crate::error::{AppError, AppResult};
use crate::integrations::{percent_encode, RemoteRepository};
use reqwest::blocking::Client;
use serde_json::json;
use std::time::Duration;

pub fn publish_review(
    config: &GitlabConfig,
    remote: &RemoteRepository,
    report: &CommitReport,
) -> AppResult<()> {
    ensure_configured(config)?;
    let client = client(config)?;
    let project = percent_encode(&remote.path);
    let merge_request = find_open_merge_request(&client, config, &project, &report.branch)?;
    let body = render_review_body(report);

    let notes_url = format!(
        "{}/projects/{}/merge_requests/{}/notes",
        config.api_url.trim_end_matches('/'),
        project,
        merge_request.iid
    );
    let response = client.post(notes_url).json(&json!({ "body": body })).send()?;
    if !response.status().is_success() {
        return Err(AppError::message(format!(
            "GitLab MR note failed with status {}.",
            response.status()
        )));
    }

    if config.update_mr_description {
        let merged = append_section(
            merge_request.description.unwrap_or_default().as_str(),
            "Gitwhisper Review Summary",
            &body,
        );
        let update_url = format!(
            "{}/projects/{}/merge_requests/{}",
            config.api_url.trim_end_matches('/'),
            project,
            merge_request.iid
        );
        let response = client
            .put(update_url)
            .json(&json!({ "description": merged }))
            .send()?;
        if !response.status().is_success() {
            return Err(AppError::message(format!(
                "GitLab MR description update failed with status {}.",
                response.status()
            )));
        }
    }

    Ok(())
}

#[derive(Debug, serde::Deserialize)]
struct GitlabMergeRequest {
    iid: u64,
    #[serde(default)]
    description: Option<String>,
}

fn find_open_merge_request(
    client: &Client,
    config: &GitlabConfig,
    project: &str,
    branch: &str,
) -> AppResult<GitlabMergeRequest> {
    let url = format!(
        "{}/projects/{}/merge_requests?state=opened&source_branch={}",
        config.api_url.trim_end_matches('/'),
        project,
        percent_encode(branch)
    );
    let response = client.get(url).send()?;
    if !response.status().is_success() {
        return Err(AppError::message(format!(
            "GitLab MR lookup failed with status {}.",
            response.status()
        )));
    }

    let mrs = response.json::<Vec<GitlabMergeRequest>>().unwrap_or_default();
    mrs.into_iter()
        .next()
        .ok_or_else(|| AppError::message(format!("No open GitLab MR found for branch `{branch}`.")))
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

fn ensure_configured(config: &GitlabConfig) -> AppResult<()> {
    if !config.enabled {
        return Err(AppError::message("GitLab integration is disabled in `.gitwhisper.toml`."));
    }

    if config.token.trim().is_empty() {
        return Err(AppError::message("GitLab integration requires `integrations.gitlab.token`."));
    }

    Ok(())
}

fn client(config: &GitlabConfig) -> AppResult<Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "PRIVATE-TOKEN",
        reqwest::header::HeaderValue::from_str(config.token.trim())
            .map_err(|_| AppError::message("Invalid GitLab token header value."))?,
    );
    Ok(Client::builder()
        .timeout(Duration::from_secs(20))
        .default_headers(headers)
        .build()?)
}

