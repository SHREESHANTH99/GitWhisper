use crate::analysis::{
    ChangeCategory, ChangeScope, DiffSummary, IntentClassification, RiskLevel, UrgencyLevel,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ConventionalCommit {
    commit_type: String,
    scope: String,
    breaking_change: bool,
}

pub fn classify_commit_intent(
    subject: &str,
    body: &str,
    diff: &DiffSummary,
) -> IntentClassification {
    let conventional = parse_conventional_commit(subject);
    let subject_lower = subject.trim().to_ascii_lowercase();
    let body_lower = body.trim().to_ascii_lowercase();
    let combined = if body_lower.is_empty() {
        subject_lower.clone()
    } else {
        format!("{subject_lower}\n{body_lower}")
    };

    let mut signals = Vec::new();
    let (category, mut confidence) = infer_category(
        &subject_lower,
        &body_lower,
        &conventional,
        diff,
        &mut signals,
    );
    let scope = infer_scope(diff);
    let urgency = infer_urgency(&combined, &category, &conventional, &mut signals);
    let breaking_change = conventional.breaking_change
        || combined.contains("breaking change")
        || combined.contains("breaking-change");
    if breaking_change {
        signals.push("breaking change marker".to_string());
    }
    let risk = infer_risk(
        &combined,
        &category,
        &scope,
        diff,
        breaking_change,
        &mut signals,
    );

    if !conventional.commit_type.is_empty() {
        signals.push("conventional commit header".to_string());
        confidence = confidence.max(90);
    }

    if signals.len() >= 3 {
        confidence = confidence.saturating_add(5).min(99);
    }

    IntentClassification {
        category,
        urgency,
        risk,
        scope,
        conventional_type: conventional.commit_type,
        conventional_scope: conventional.scope,
        breaking_change,
        signals,
        confidence,
    }
}

fn infer_category(
    subject: &str,
    body: &str,
    conventional: &ConventionalCommit,
    diff: &DiffSummary,
    signals: &mut Vec<String>,
) -> (ChangeCategory, u8) {
    if let Some(category) = category_from_conventional(&conventional.commit_type) {
        signals.push(format!("type `{}`", conventional.commit_type));
        return (category, 94);
    }

    let combined = if body.is_empty() {
        subject.to_string()
    } else {
        format!("{subject}\n{body}")
    };

    if contains_any(&combined, &["fix", "bug", "hotfix", "patch"]) {
        signals.push("bug-fix wording".to_string());
        return (ChangeCategory::BugFix, 88);
    }
    if contains_any(
        &combined,
        &["feat", "feature", "implement", "introduce", "add "],
    ) {
        signals.push("feature wording".to_string());
        return (ChangeCategory::Feature, 86);
    }
    if contains_any(
        &combined,
        &["refactor", "cleanup", "restructure", "simplify"],
    ) {
        signals.push("refactor wording".to_string());
        return (ChangeCategory::Refactor, 85);
    }
    if contains_any(
        &combined,
        &["perf", "optimize", "speed", "latency", "cache"],
    ) {
        signals.push("performance wording".to_string());
        return (ChangeCategory::Performance, 84);
    }
    if contains_any(&combined, &["docs", "documentation", "readme", "comment"]) {
        signals.push("documentation wording".to_string());
        return (ChangeCategory::Documentation, 84);
    }
    if contains_any(
        &combined,
        &["deps", "dependency", "dependencies", "upgrade", "bump"],
    ) || diff.import_changes.len() >= 3
    {
        signals.push("dependency/update wording".to_string());
        return (ChangeCategory::DependencyUpdate, 80);
    }
    if contains_any(&combined, &["test", "tests", "spec"])
        || all_files_match(diff, &["test", "spec"])
    {
        signals.push("test-focused wording".to_string());
        return (ChangeCategory::Test, 80);
    }
    if contains_any(&combined, &["chore", "ci", "build", "release"]) {
        signals.push("maintenance wording".to_string());
        return (ChangeCategory::Chore, 78);
    }
    if all_files_match(diff, &["md", "txt", "rst"]) {
        signals.push("docs-like files only".to_string());
        return (ChangeCategory::Documentation, 70);
    }

    (ChangeCategory::Unknown, 35)
}

fn infer_urgency(
    combined: &str,
    category: &ChangeCategory,
    conventional: &ConventionalCommit,
    signals: &mut Vec<String>,
) -> UrgencyLevel {
    if contains_any(
        combined,
        &[
            "hotfix", "critical", "sev1", "sev-1", "outage", "incident", "rollback",
        ],
    ) {
        signals.push("critical production keyword".to_string());
        return UrgencyLevel::Critical;
    }

    if conventional.breaking_change
        || contains_any(combined, &["urgent", "asap", "security", "vulnerability"])
    {
        signals.push("high-priority keyword".to_string());
        return UrgencyLevel::High;
    }

    if matches!(
        category,
        ChangeCategory::Documentation | ChangeCategory::Chore | ChangeCategory::Test
    ) {
        return UrgencyLevel::Low;
    }

    UrgencyLevel::Normal
}

fn infer_scope(diff: &DiffSummary) -> ChangeScope {
    match diff.files_changed {
        0 | 1 => ChangeScope::SingleFile,
        2..=5 => {
            if diff.files_added + diff.files_deleted + diff.files_renamed >= 3
                || diff.symbol_changes.len() >= 8
            {
                ChangeScope::Broad
            } else {
                ChangeScope::CrossFile
            }
        }
        6.. => ChangeScope::Broad,
    }
}

fn infer_risk(
    combined: &str,
    category: &ChangeCategory,
    scope: &ChangeScope,
    diff: &DiffSummary,
    breaking_change: bool,
    signals: &mut Vec<String>,
) -> RiskLevel {
    let destructive_ops = diff.files_deleted + diff.files_renamed;
    let symbol_count = diff.symbol_changes.len();
    let import_count = diff.import_changes.len();
    let complexity = diff.complexity_delta.abs();

    if breaking_change
        || contains_any(
            combined,
            &["migration", "schema", "api", "auth", "security"],
        ) && matches!(scope, ChangeScope::Broad)
    {
        signals.push("breaking or system-level keyword".to_string());
        return RiskLevel::Critical;
    }

    if destructive_ops > 0
        || complexity >= 5
        || symbol_count >= 6
        || import_count >= 4
        || matches!(scope, ChangeScope::Broad)
    {
        if destructive_ops > 0 {
            signals.push("rename/delete operations".to_string());
        }
        if complexity >= 5 {
            signals.push("complexity increase".to_string());
        }
        if symbol_count >= 6 {
            signals.push("many symbols touched".to_string());
        }
        if import_count >= 4 {
            signals.push("many import changes".to_string());
        }
        return RiskLevel::High;
    }

    if matches!(
        category,
        ChangeCategory::Documentation | ChangeCategory::Chore | ChangeCategory::Test
    ) && destructive_ops == 0
        && complexity == 0
        && symbol_count <= 1
    {
        signals.push("low-impact maintenance pattern".to_string());
        return RiskLevel::Low;
    }

    RiskLevel::Medium
}

fn parse_conventional_commit(subject: &str) -> ConventionalCommit {
    let header = subject.trim();
    let Some((prefix, _description)) = header.split_once(':') else {
        return ConventionalCommit::default();
    };

    let prefix = prefix.trim();
    if prefix.is_empty() || prefix.contains(' ') {
        return ConventionalCommit::default();
    }

    let breaking_change = prefix.ends_with('!');
    let prefix = prefix.trim_end_matches('!');

    if let Some((commit_type, rest)) = prefix.split_once('(') {
        let scope = rest.strip_suffix(')').unwrap_or_default().trim();
        if commit_type.trim().is_empty() || scope.is_empty() {
            return ConventionalCommit::default();
        }

        ConventionalCommit {
            commit_type: commit_type.trim().to_string(),
            scope: scope.to_string(),
            breaking_change,
        }
    } else {
        ConventionalCommit {
            commit_type: prefix.to_string(),
            scope: String::new(),
            breaking_change,
        }
    }
}

fn category_from_conventional(commit_type: &str) -> Option<ChangeCategory> {
    match commit_type {
        "fix" | "hotfix" | "patch" => Some(ChangeCategory::BugFix),
        "feat" | "feature" => Some(ChangeCategory::Feature),
        "refactor" => Some(ChangeCategory::Refactor),
        "perf" => Some(ChangeCategory::Performance),
        "docs" | "doc" => Some(ChangeCategory::Documentation),
        "deps" | "dep" | "build" => Some(ChangeCategory::DependencyUpdate),
        "test" | "tests" => Some(ChangeCategory::Test),
        "chore" | "ci" | "release" => Some(ChangeCategory::Chore),
        _ => None,
    }
}

fn contains_any(input: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| input.contains(needle))
}

fn all_files_match(diff: &DiffSummary, extensions: &[&str]) -> bool {
    !diff.file_stats.is_empty()
        && diff.file_stats.iter().all(|file| {
            let extension = file.path.rsplit('.').next().unwrap_or_default();
            extensions
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
                || file.path.to_ascii_lowercase().contains("/test")
                || file.path.to_ascii_lowercase().contains("\\test")
        })
}

#[cfg(test)]
mod tests {
    use super::classify_commit_intent;
    use crate::analysis::{
        ChangeCategory, ChangeScope, DiffSummary, FileDiffStat, FileOperation, RiskLevel,
        UrgencyLevel,
    };

    #[test]
    fn detects_bug_fix_commits() {
        let diff = DiffSummary {
            files_changed: 2,
            ..DiffSummary::default()
        };
        let intent = classify_commit_intent("fix(auth): reject expired tokens", "", &diff);
        assert_eq!(intent.category, ChangeCategory::BugFix);
        assert_eq!(intent.scope, ChangeScope::CrossFile);
        assert_eq!(intent.urgency, UrgencyLevel::Normal);
        assert_eq!(intent.risk, RiskLevel::Medium);
        assert_eq!(intent.conventional_type, "fix");
        assert_eq!(intent.conventional_scope, "auth");
    }

    #[test]
    fn marks_hotfixes_as_critical() {
        let diff = DiffSummary {
            files_changed: 1,
            ..DiffSummary::default()
        };
        let intent = classify_commit_intent("hotfix: patch auth bypass", "", &diff);
        assert_eq!(intent.category, ChangeCategory::BugFix);
        assert_eq!(intent.urgency, UrgencyLevel::Critical);
        assert_eq!(intent.scope, ChangeScope::SingleFile);
    }

    #[test]
    fn detects_breaking_feature_with_high_risk() {
        let diff = DiffSummary {
            files_changed: 6,
            files_deleted: 1,
            files_renamed: 1,
            complexity_delta: 7,
            ..DiffSummary::default()
        };

        let intent = classify_commit_intent(
            "feat(api)!: replace auth contract",
            "BREAKING CHANGE: clients must send refresh tokens",
            &diff,
        );

        assert_eq!(intent.category, ChangeCategory::Feature);
        assert_eq!(intent.urgency, UrgencyLevel::High);
        assert_eq!(intent.risk, RiskLevel::Critical);
        assert_eq!(intent.scope, ChangeScope::Broad);
        assert!(intent.breaking_change);
    }

    #[test]
    fn treats_doc_only_changes_as_low_risk() {
        let diff = DiffSummary {
            files_changed: 1,
            file_stats: vec![FileDiffStat {
                path: "README.md".to_string(),
                operation: FileOperation::Modified,
                ..FileDiffStat::default()
            }],
            ..DiffSummary::default()
        };

        let intent = classify_commit_intent("docs: clarify setup", "", &diff);
        assert_eq!(intent.category, ChangeCategory::Documentation);
        assert_eq!(intent.urgency, UrgencyLevel::Low);
        assert_eq!(intent.risk, RiskLevel::Low);
    }
}
