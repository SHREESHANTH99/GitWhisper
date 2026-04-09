use crate::analysis::{ChangeCategory, ChangeScope, IntentClassification, UrgencyLevel};

pub fn classify_commit_message(message: &str, files_changed: usize) -> IntentClassification {
    let normalized = message.trim().to_ascii_lowercase();

    let (category, confidence) = if starts_with_any(&normalized, &["fix", "hotfix", "patch"]) {
        (ChangeCategory::BugFix, 92)
    } else if starts_with_any(&normalized, &["feat", "feature", "add"]) {
        (ChangeCategory::Feature, 90)
    } else if starts_with_any(&normalized, &["refactor", "cleanup", "improve"]) {
        (ChangeCategory::Refactor, 88)
    } else if starts_with_any(&normalized, &["perf", "optimize", "speed"]) {
        (ChangeCategory::Performance, 88)
    } else if starts_with_any(&normalized, &["docs", "doc", "comment"]) {
        (ChangeCategory::Documentation, 86)
    } else if starts_with_any(
        &normalized,
        &["deps", "dependency", "dependencies", "upgrade", "bump"],
    ) {
        (ChangeCategory::DependencyUpdate, 84)
    } else if starts_with_any(&normalized, &["test", "tests"]) {
        (ChangeCategory::Test, 84)
    } else if starts_with_any(&normalized, &["chore"]) {
        (ChangeCategory::Chore, 80)
    } else {
        (ChangeCategory::Unknown, 35)
    };

    let urgency = if contains_any(&normalized, &["hotfix", "critical", "sev1", "sev-1"]) {
        UrgencyLevel::Critical
    } else if contains_any(&normalized, &["urgent", "asap", "priority"]) {
        UrgencyLevel::High
    } else if matches!(
        category,
        ChangeCategory::Documentation | ChangeCategory::Chore
    ) {
        UrgencyLevel::Low
    } else {
        UrgencyLevel::Normal
    };

    let scope = match files_changed {
        0 | 1 => ChangeScope::SingleFile,
        2..=5 => ChangeScope::CrossFile,
        6.. => ChangeScope::Broad,
    };

    IntentClassification {
        category,
        urgency,
        scope,
        confidence,
    }
}

fn starts_with_any(input: &str, prefixes: &[&str]) -> bool {
    prefixes
        .iter()
        .any(|prefix| input.starts_with(prefix) || input.starts_with(&format!("{prefix}(")))
}

fn contains_any(input: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| input.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::classify_commit_message;
    use crate::analysis::{ChangeCategory, ChangeScope, UrgencyLevel};

    #[test]
    fn detects_bug_fix_commits() {
        let intent = classify_commit_message("fix(auth): reject expired tokens", 2);
        assert_eq!(intent.category, ChangeCategory::BugFix);
        assert_eq!(intent.scope, ChangeScope::CrossFile);
        assert_eq!(intent.urgency, UrgencyLevel::Normal);
    }

    #[test]
    fn marks_hotfixes_as_critical() {
        let intent = classify_commit_message("hotfix: patch auth bypass", 1);
        assert_eq!(intent.category, ChangeCategory::BugFix);
        assert_eq!(intent.urgency, UrgencyLevel::Critical);
        assert_eq!(intent.scope, ChangeScope::SingleFile);
    }
}
