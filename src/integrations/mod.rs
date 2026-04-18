pub mod discord;
pub mod github;
pub mod gitlab;
pub mod slack;

use crate::collaboration::CommitReport;
use crate::config::AppConfig;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostingProvider {
    Github,
    Gitlab,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct RemoteRepository {
    pub provider: HostingProvider,
    pub host: String,
    pub path: String,
    pub owner: String,
    pub repo: String,
}

pub fn auto_deliver_commit_report(config: &AppConfig, report: &CommitReport) -> AppResult<()> {
    let remote = current_remote_repository();
    let commit_url = remote
        .as_ref()
        .and_then(|repo| commit_url(repo, &report.full_commit));
    let mut errors = Vec::new();

    if config.integrations.slack.enabled && config.integrations.slack.auto_share_on_commit {
        if let Err(error) = slack::send_commit(&config.integrations.slack, report, commit_url.as_deref()) {
            errors.push(format!("slack: {error}"));
        }
    }

    if config.integrations.discord.enabled && config.integrations.discord.auto_share_on_commit {
        if let Err(error) =
            discord::send_commit(&config.integrations.discord, report, commit_url.as_deref())
        {
            errors.push(format!("discord: {error}"));
        }
    }

    if config.integrations.github.enabled && config.integrations.github.auto_comment_on_pr {
        if let Some(remote) = remote.as_ref().filter(|repo| repo.provider == HostingProvider::Github)
        {
            if let Err(error) = github::publish_review(&config.integrations.github, remote, report) {
                errors.push(format!("github: {error}"));
            }
        }
    }

    if config.integrations.gitlab.enabled && config.integrations.gitlab.auto_comment_on_mr {
        if let Some(remote) = remote.as_ref().filter(|repo| repo.provider == HostingProvider::Gitlab)
        {
            if let Err(error) = gitlab::publish_review(&config.integrations.gitlab, remote, report) {
                errors.push(format!("gitlab: {error}"));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(AppError::message(errors.join(" | ")))
    }
}

pub fn share_commit(provider: &str, commit: Option<&str>, api_key: &str) {
    match share_commit_inner(provider, commit, api_key) {
        Ok(message) => println!("{message}"),
        Err(error) => eprintln!("{error}"),
    }
}

pub fn publish_review(provider: &str, commit: Option<&str>, api_key: &str) {
    match publish_review_inner(provider, commit, api_key) {
        Ok(message) => println!("{message}"),
        Err(error) => eprintln!("{error}"),
    }
}

pub fn send_digest(provider: &str, period: &str) {
    match send_digest_inner(provider, period) {
        Ok(message) => println!("{message}"),
        Err(error) => eprintln!("{error}"),
    }
}

fn share_commit_inner(provider: &str, commit: Option<&str>, api_key: &str) -> AppResult<String> {
    let config = AppConfig::load()?;
    let report = crate::collaboration::prepare_commit_report(commit, api_key, false)?;
    let remote = current_remote_repository();
    let commit_url = remote
        .as_ref()
        .and_then(|repo| commit_url(repo, &report.full_commit));

    match provider {
        "slack" => {
            slack::send_commit(&config.integrations.slack, &report, commit_url.as_deref())?;
            Ok(format!("Shared commit {} to Slack.", report.commit))
        }
        "discord" => {
            discord::send_commit(&config.integrations.discord, &report, commit_url.as_deref())?;
            Ok(format!("Shared commit {} to Discord.", report.commit))
        }
        other => Err(AppError::message(format!(
            "Unsupported share provider `{other}`. Use `slack` or `discord`."
        ))),
    }
}

fn publish_review_inner(provider: &str, commit: Option<&str>, api_key: &str) -> AppResult<String> {
    let config = AppConfig::load()?;
    let report = crate::collaboration::prepare_commit_report(commit, api_key, false)?;
    let remote = current_remote_repository()
        .ok_or_else(|| AppError::message("Could not resolve the repository remote from `origin`."))?;

    match provider {
        "github" => {
            github::publish_review(&config.integrations.github, &remote, &report)?;
            Ok(format!("Published review note for commit {} to GitHub PR.", report.commit))
        }
        "gitlab" => {
            gitlab::publish_review(&config.integrations.gitlab, &remote, &report)?;
            Ok(format!("Published review note for commit {} to GitLab MR.", report.commit))
        }
        other => Err(AppError::message(format!(
            "Unsupported review provider `{other}`. Use `github` or `gitlab`."
        ))),
    }
}

fn send_digest_inner(provider: &str, period: &str) -> AppResult<String> {
    let config = AppConfig::load()?;
    let digest = crate::metrics::build_digest(period)?;

    match provider {
        "slack" => {
            slack::send_digest(&config.integrations.slack, period, &digest)?;
            Ok(format!("Sent {} digest to Slack.", period))
        }
        "discord" => {
            discord::send_digest(&config.integrations.discord, period, &digest)?;
            Ok(format!("Sent {} digest to Discord.", period))
        }
        other => Err(AppError::message(format!(
            "Unsupported digest provider `{other}`. Use `slack` or `discord`."
        ))),
    }
}

pub fn current_remote_repository() -> Option<RemoteRepository> {
    let remote = crate::git::remote_url("origin").ok()?;
    parse_remote_url(&remote)
}

pub fn commit_url(repo: &RemoteRepository, commit: &str) -> Option<String> {
    if repo.path.is_empty() {
        return None;
    }

    let base = format!("https://{}/{}", repo.host, repo.path);
    Some(match repo.provider {
        HostingProvider::Github => format!("{base}/commit/{commit}"),
        HostingProvider::Gitlab => format!("{base}/-/commit/{commit}"),
        HostingProvider::Unknown => format!("{base}/commit/{commit}"),
    })
}

pub fn parse_remote_url(input: &str) -> Option<RemoteRepository> {
    let trimmed = input.trim().trim_end_matches(".git");
    let (host, path) = if let Some(rest) = trimmed.strip_prefix("https://") {
        let (host, path) = rest.split_once('/')?;
        (host.to_string(), path.to_string())
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        let (host, path) = rest.split_once('/')?;
        (host.to_string(), path.to_string())
    } else if let Some(rest) = trimmed.strip_prefix("git@") {
        let (host, path) = rest.split_once(':')?;
        (host.to_string(), path.to_string())
    } else {
        return None;
    };

    let segments = path.split('/').collect::<Vec<_>>();
    if segments.len() < 2 {
        return None;
    }

    let owner = segments.first()?.to_string();
    let repo = segments.last()?.to_string();
    let provider = if host.contains("github") {
        HostingProvider::Github
    } else if host.contains("gitlab") {
        HostingProvider::Gitlab
    } else {
        HostingProvider::Unknown
    };

    Some(RemoteRepository {
        provider,
        host,
        path,
        owner,
        repo,
    })
}

pub fn percent_encode(input: &str) -> String {
    let mut output = String::new();
    for byte in input.bytes() {
        let ch = byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
            output.push(ch);
        } else {
            output.push_str(&format!("%{:02X}", byte));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{parse_remote_url, HostingProvider};

    #[test]
    fn parses_https_github_remote() {
        let remote = parse_remote_url("https://github.com/openai/example.git").unwrap();
        assert_eq!(remote.provider, HostingProvider::Github);
        assert_eq!(remote.owner, "openai");
        assert_eq!(remote.repo, "example");
    }

    #[test]
    fn parses_ssh_gitlab_remote() {
        let remote = parse_remote_url("git@gitlab.com:group/sub/project.git").unwrap();
        assert_eq!(remote.provider, HostingProvider::Gitlab);
        assert_eq!(remote.repo, "project");
    }
}

