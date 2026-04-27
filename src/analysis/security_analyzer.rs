use crate::analysis::codebase_insights;
use crate::error::AppResult;

const MAX_FILES_PER_REPORT: usize = 12;

#[derive(Debug, Clone)]
pub struct SecurityReport {
    pub target: String,
    pub files_analyzed: usize,
    pub overall_risk: u32,
    pub file_reports: Vec<FileSecurityReport>,
    pub findings: Vec<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FileSecurityReport {
    pub path: String,
    pub risk_score: u32,
    pub secret_hits: usize,
    pub injection_hits: usize,
    pub shell_execution_hits: usize,
    pub crypto_hits: usize,
    pub auth_hits: usize,
    pub findings: Vec<String>,
    pub suggestions: Vec<String>,
}

pub fn analyze_target(path: &str) -> AppResult<SecurityReport> {
    let (root, normalized) = codebase_insights::normalize_target(path)?;
    let files = codebase_insights::collect_target_files(&root, &normalized)?;
    let is_file = files.len() == 1 && files[0] == normalized;

    let mut file_reports = files
        .iter()
        .filter_map(|file| analyze_file(&root, file).ok())
        .collect::<Vec<_>>();
    file_reports.sort_by(|left, right| {
        right
            .risk_score
            .cmp(&left.risk_score)
            .then_with(|| left.path.cmp(&right.path))
    });

    let files_analyzed = file_reports.len();
    let overall_risk = average_risk(&file_reports);

    if is_file {
        let report = file_reports.into_iter().next().expect("single file report exists");
        return Ok(SecurityReport {
            target: normalized,
            files_analyzed: 1,
            overall_risk: report.risk_score,
            findings: report.findings.clone(),
            suggestions: report.suggestions.clone(),
            file_reports: vec![report],
        });
    }

    let findings = build_findings(&file_reports);
    let suggestions = build_suggestions(&file_reports);
    if file_reports.len() > MAX_FILES_PER_REPORT {
        file_reports.truncate(MAX_FILES_PER_REPORT);
    }

    Ok(SecurityReport {
        target: normalized,
        files_analyzed,
        overall_risk,
        file_reports,
        findings,
        suggestions,
    })
}

fn analyze_file(root: &std::path::Path, normalized_file: &str) -> AppResult<FileSecurityReport> {
    let insight = codebase_insights::analyze_file(root, normalized_file)?;
    Ok(report_for_insight(normalized_file, &insight))
}

pub(crate) fn report_for_insight(
    normalized_file: &str,
    insight: &codebase_insights::FileInsight,
) -> FileSecurityReport {
    let lowered = insight.content.to_ascii_lowercase();

    let secret_hits = count_lines(&lowered, |line| {
        let looks_assignment = line.contains('=') || line.contains(':');
        looks_assignment
            && contains_any(
                line,
                &["api_key", "secret_key", "password", "token", "private_key", "aws_secret_access_key"],
            )
            && !is_pattern_literal(line)
    });
    let injection_hits = count_lines(&lowered, |line| {
        (contains_any(
            line,
            &["select ", "insert into", "update ", "delete from", "query(", "execute("],
        ) && contains_any(line, &["format!", "+", "{", "$", "%s"])
            && !is_pattern_literal(line))
            || contains_any(line, &["raw_query", "unsafe_sql"])
    });
    let shell_execution_hits = count_lines(&lowered, |line| {
        contains_any(line, &["command::new(", "exec(", "system(", "popen(", "spawn(", "subprocess."])
            && !is_pattern_literal(line)
    });
    let crypto_hits = count_lines(&lowered, |line| {
        contains_any(line, &["md5", "sha1", "des", "rc4", "math.random", "weak_rng", "insecure"])
            && !is_pattern_literal(line)
    });
    let auth_hits = count_lines(&lowered, |line| {
        contains_any(line, &["auth", "token", "jwt", "session", "password", "oauth", "permission"])
            && !is_pattern_literal(line)
    });

    let mut findings = Vec::new();
    let mut suggestions = Vec::new();
    let mut risk_score = 0u32;

    if secret_hits > 0 {
        findings.push(format!(
            "{} contains {} possible hardcoded secret or credential markers.",
            normalized_file, secret_hits
        ));
        suggestions.push(format!(
            "Move secrets in `{}` to environment variables or a managed secret store.",
            normalized_file
        ));
        risk_score += 28;
    }

    if injection_hits > 0 {
        findings.push(format!(
            "{} shows {} possible query-construction or injection-sensitive patterns.",
            normalized_file, injection_hits
        ));
        suggestions.push(format!(
            "Review data-access code in `{}` for parameterized queries and input sanitization.",
            normalized_file
        ));
        risk_score += 24;
    }

    if shell_execution_hits > 0 {
        findings.push(format!(
            "{} invokes shell or process execution {} time(s).",
            normalized_file, shell_execution_hits
        ));
        suggestions.push(format!(
            "Harden command execution in `{}` by validating inputs and avoiding shell interpolation.",
            normalized_file
        ));
        risk_score += 16;
    }

    if crypto_hits > 0 {
        findings.push(format!(
            "{} references {} potentially weak crypto or randomization markers.",
            normalized_file, crypto_hits
        ));
        suggestions.push(format!(
            "Review `{}` for modern cryptography and secure randomness usage.",
            normalized_file
        ));
        risk_score += 18;
    }

    if auth_hits >= 6 && insight.bug_fix_commits >= 2 {
        findings.push(format!(
            "{} is a sensitive auth-related file with repeated bug-fix history.",
            normalized_file
        ));
        suggestions.push(format!(
            "Increase security review depth and regression coverage for `{}`.",
            normalized_file
        ));
        risk_score += 16;
    }

    if insight.recent_churn >= 100 && auth_hits > 0 {
        findings.push(format!(
            "{} mixes security-sensitive logic with high recent churn.",
            normalized_file
        ));
        risk_score += 10;
    }

    if findings.is_empty() {
        findings.push(format!(
            "{} has no strong security red flags from current static heuristics.",
            normalized_file
        ));
        suggestions.push(format!(
            "Keep `{}` under review as more security-sensitive history accumulates.",
            normalized_file
        ));
    }

    FileSecurityReport {
        path: normalized_file.to_string(),
        risk_score: risk_score.min(100),
        secret_hits,
        injection_hits,
        shell_execution_hits,
        crypto_hits,
        auth_hits,
        findings,
        suggestions,
    }
}

fn count_lines(content: &str, predicate: impl Fn(&str) -> bool) -> usize {
    content.lines().filter(|line| predicate(line.trim())).count()
}

fn contains_any(line: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| line.contains(pattern))
}

fn is_pattern_literal(line: &str) -> bool {
    let trimmed = line.trim();
    (trimmed.starts_with('"') || trimmed.starts_with("&[") || trimmed.starts_with('['))
        && !trimmed.contains('=')
        && !trimmed.contains(':')
}

fn average_risk(file_reports: &[FileSecurityReport]) -> u32 {
    if file_reports.is_empty() {
        return 0;
    }

    let total: u32 = file_reports.iter().map(|report| report.risk_score).sum();
    total / file_reports.len() as u32
}

fn build_findings(file_reports: &[FileSecurityReport]) -> Vec<String> {
    let risky_files = file_reports
        .iter()
        .filter(|report| report.risk_score >= 40)
        .map(|report| report.path.clone())
        .take(4)
        .collect::<Vec<_>>();
    if risky_files.is_empty() {
        return vec!["No broad directory-level security hotspot stands out from current heuristics.".to_string()];
    }

    vec![format!(
        "Security-sensitive hotspots are concentrated in {}.",
        risky_files.join(", ")
    )]
}

fn build_suggestions(file_reports: &[FileSecurityReport]) -> Vec<String> {
    let top = file_reports.iter().take(3).collect::<Vec<_>>();
    if top.is_empty() {
        return vec!["Keep collecting commit context to improve future security reports.".to_string()];
    }

    vec![format!(
        "Start security review with {} because they currently carry the highest heuristic risk.",
        top.iter()
            .map(|report| report.path.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )]
}

#[cfg(test)]
mod tests {
    use super::{contains_any, count_lines, is_pattern_literal};

    #[test]
    fn counts_security_markers() {
        let content = "api_key=1\npassword = 2\nCommand::new(\"sh\")";
        let lowered = content.to_ascii_lowercase();
        assert_eq!(count_lines(&lowered, |line| contains_any(line, &["api_key", "password", "command::new("])), 3);
    }

    #[test]
    fn ignores_pattern_literal_lines() {
        assert!(is_pattern_literal("\"api_key\","));
    }
}
