use crate::storage::context::ReviewContext;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

pub fn collect_review_context(commands: &[String]) -> ReviewContext {
    let mut review = ReviewContext {
        ci_provider: detect_ci_provider(),
        ..ReviewContext::default()
    };

    if let Some(github_review) = github_pr_context() {
        review = merge_review_context(review, github_review);
    }

    if let Some(gitlab_review) = gitlab_pr_context() {
        review = merge_review_context(review, gitlab_review);
    }

    if let Some((tests_run, tests_failed, coverage_percent, source)) = local_reports() {
        review.tests_run = tests_run;
        review.tests_failed = tests_failed;
        review.coverage_percent = coverage_percent;
        review.source = source;
        review.test_status = if tests_run > 0 && tests_failed == 0 {
            "passed".to_string()
        } else if tests_failed > 0 {
            "failed".to_string()
        } else {
            "unknown".to_string()
        };
    } else if commands.iter().any(|command| command.contains("test")) {
        review.test_status = "requested".to_string();
    }

    review
}

fn merge_review_context(mut base: ReviewContext, incoming: ReviewContext) -> ReviewContext {
    if base.ci_provider.is_empty() {
        base.ci_provider = incoming.ci_provider;
    }
    if base.pr_number.is_empty() {
        base.pr_number = incoming.pr_number;
    }
    if base.reviewers.is_empty() {
        base.reviewers = incoming.reviewers;
    }
    if base.labels.is_empty() {
        base.labels = incoming.labels;
    }
    if base.milestone.is_empty() {
        base.milestone = incoming.milestone;
    }
    if base.source.is_empty() {
        base.source = incoming.source;
    }
    base
}

fn detect_ci_provider() -> String {
    if std::env::var("GITHUB_ACTIONS").ok().as_deref() == Some("true") {
        "github-actions".to_string()
    } else if std::env::var("GITLAB_CI").ok().as_deref() == Some("true") {
        "gitlab-ci".to_string()
    } else if std::env::var("JENKINS_URL").is_ok() {
        "jenkins".to_string()
    } else if std::env::var("CIRCLECI").ok().as_deref() == Some("true") {
        "circleci".to_string()
    } else {
        String::new()
    }
}

fn github_pr_context() -> Option<ReviewContext> {
    let event_path = std::env::var("GITHUB_EVENT_PATH").ok()?;
    let raw = fs::read_to_string(event_path).ok()?;
    let json = serde_json::from_str::<serde_json::Value>(&raw).ok()?;
    let pull_request = json.get("pull_request")?;

    Some(ReviewContext {
        ci_provider: "github-actions".to_string(),
        pr_number: pull_request
            .get("number")
            .or_else(|| json.get("number"))
            .and_then(|value| value.as_i64())
            .map(|value| value.to_string())
            .unwrap_or_default(),
        reviewers: pull_request
            .get("requested_reviewers")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("login").and_then(|value| value.as_str()))
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        labels: pull_request
            .get("labels")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("name").and_then(|value| value.as_str()))
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        milestone: pull_request
            .get("milestone")
            .and_then(|value| value.get("title"))
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        source: "github-event".to_string(),
        ..ReviewContext::default()
    })
}

fn gitlab_pr_context() -> Option<ReviewContext> {
    let pr_number = std::env::var("CI_MERGE_REQUEST_IID").ok()?;
    Some(ReviewContext {
        ci_provider: "gitlab-ci".to_string(),
        pr_number,
        source: "gitlab-env".to_string(),
        ..ReviewContext::default()
    })
}

fn local_reports() -> Option<(usize, usize, Option<u8>, String)> {
    let root = crate::git::repo_root().ok()?;
    let report_files = find_report_files(&root);

    let mut tests_run = 0usize;
    let mut tests_failed = 0usize;
    let mut coverage_percent = None;
    let mut source = String::new();

    for report in report_files {
        let file_name = report.file_name()?.to_string_lossy().to_ascii_lowercase();
        let raw = fs::read_to_string(&report).ok()?;

        if file_name == "coverage-summary.json" {
            coverage_percent = parse_istanbul_coverage(&raw).or(coverage_percent);
            source = "coverage-summary.json".to_string();
        } else if file_name.ends_with(".info") {
            coverage_percent = parse_lcov_coverage(&raw).or(coverage_percent);
            source = "lcov.info".to_string();
        } else if file_name.ends_with(".xml") {
            let (run, failed) = parse_junit_counts(&raw);
            if run > 0 {
                tests_run = run;
                tests_failed = failed;
                source = file_name.clone();
            }
            coverage_percent = parse_cobertura_coverage(&raw).or(coverage_percent);
        }
    }

    if tests_run == 0 && coverage_percent.is_none() {
        None
    } else {
        Some((tests_run, tests_failed, coverage_percent, source))
    }
}

fn find_report_files(root: &PathBuf) -> Vec<PathBuf> {
    let names = [
        "coverage-summary.json",
        "lcov.info",
        "junit.xml",
        "test-results.xml",
        "coverage.xml",
        "cobertura.xml",
    ];

    WalkDir::new(root)
        .max_depth(4)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.path().to_path_buf())
        .filter(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .map(|name| {
                    names
                        .iter()
                        .any(|candidate| candidate.eq_ignore_ascii_case(name))
                })
                .unwrap_or(false)
        })
        .collect()
}

fn parse_istanbul_coverage(raw: &str) -> Option<u8> {
    let json = serde_json::from_str::<serde_json::Value>(raw).ok()?;
    json.get("total")
        .and_then(|value| value.get("lines"))
        .and_then(|value| value.get("pct"))
        .and_then(|value| value.as_f64())
        .map(|value| value.round() as u8)
}

fn parse_lcov_coverage(raw: &str) -> Option<u8> {
    let mut found = 0u32;
    let mut hit = 0u32;

    for line in raw.lines() {
        if let Some(value) = line.strip_prefix("LF:") {
            found += value.trim().parse::<u32>().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("LH:") {
            hit += value.trim().parse::<u32>().unwrap_or(0);
        }
    }

    if found == 0 {
        None
    } else {
        Some(((hit as f32 / found as f32) * 100.0).round() as u8)
    }
}

fn parse_junit_counts(raw: &str) -> (usize, usize) {
    let tests_run = raw.matches("<testcase").count();
    let tests_failed = raw.matches("<failure").count() + raw.matches("<error").count();
    (tests_run, tests_failed)
}

fn parse_cobertura_coverage(raw: &str) -> Option<u8> {
    let marker = "line-rate=\"";
    let start = raw.find(marker)? + marker.len();
    let end = raw[start..].find('"')? + start;
    let rate = raw[start..end].parse::<f32>().ok()?;
    Some((rate * 100.0).round() as u8)
}

#[cfg(test)]
mod tests {
    use super::{parse_istanbul_coverage, parse_junit_counts, parse_lcov_coverage};

    #[test]
    fn parses_coverage_formats() {
        assert_eq!(
            parse_istanbul_coverage(r#"{"total":{"lines":{"pct":91.4}}}"#),
            Some(91)
        );
        assert_eq!(parse_lcov_coverage("LF:10\nLH:8\n"), Some(80));
    }

    #[test]
    fn parses_junit_counts() {
        let raw = r#"<testsuite><testcase/><testcase><failure/></testcase></testsuite>"#;
        assert_eq!(parse_junit_counts(raw), (2, 1));
    }
}
