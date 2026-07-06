//! Shared terminal-output helpers used by all viewer modules.
//!
//! Every public function in this module returns either a `colored::ColoredString`
//! (for callers that embed the value inside a larger `format!`) or a `String`
//! (for pre-built multi-char composites like bars and badges).

use colored::Colorize;

// ── Width constants ───────────────────────────────────────────────────────────
const RULE_WIDTH: usize = 62;

// ── Section header ────────────────────────────────────────────────────────────

/// Prints a visual header block:
/// ```text
/// ──────────────────────────────────────────────────────────────
/// 🔍 Title  subtitle
/// ──────────────────────────────────────────────────────────────
/// ```
pub fn section_header(icon: &str, title: &str, subtitle: &str) {
    let rule = "─".repeat(RULE_WIDTH);
    println!("{}", rule.bright_black());
    if subtitle.is_empty() {
        println!("{} {}", icon, title.bold().bright_white());
    } else {
        println!(
            "{} {}  {}",
            icon,
            title.bold().bright_white(),
            subtitle.cyan().bold()
        );
    }
    println!("{}", rule.bright_black());
}

/// Prints a lightweight sub-section label (no horizontal rules).
pub fn subsection(label: &str) {
    println!(
        "\n  {} {}",
        "›".bright_black(),
        label.bold().bright_white()
    );
}

// ── Risk helpers ──────────────────────────────────────────────────────────────

/// Returns a `ColoredString` of `"{score}/100"` coloured by severity.
/// * 0–39  → green  (low risk)
/// * 40–69 → yellow (moderate)
/// * 70+   → red    (high)
pub fn risk_colored(score: u32) -> colored::ColoredString {
    let text = format!("{score}/100");
    match score {
        0..=39 => text.green().bold(),
        40..=69 => text.yellow().bold(),
        _ => text.red().bold(),
    }
}

/// Returns a 10-cell `████░░░░░░` bar coloured by severity.
pub fn risk_bar(score: u32) -> String {
    let clamped = score.min(100);
    let filled = (clamped / 10) as usize;
    let empty = 10usize.saturating_sub(filled);
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
    match clamped {
        0..=39 => bar.green().to_string(),
        40..=69 => bar.yellow().to_string(),
        _ => bar.red().to_string(),
    }
}

// ── Intent badge ──────────────────────────────────────────────────────────────

/// Returns a coloured `[intent]` label suitable for inline use.
pub fn intent_badge(intent: &str) -> String {
    let label = format!("[{intent}]");
    match intent.to_ascii_lowercase().as_str() {
        "feature" | "feat"        => label.green().bold().to_string(),
        "fix" | "bugfix" | "bug" => label.red().bold().to_string(),
        "refactor"                => label.cyan().bold().to_string(),
        "performance" | "perf"   => label.yellow().bold().to_string(),
        "security"                => label.magenta().bold().to_string(),
        "docs" | "documentation" => label.blue().bold().to_string(),
        _                         => label.bright_black().bold().to_string(),
    }
}

// ── Commit SHA ────────────────────────────────────────────────────────────────

/// Colours a short commit SHA in amber / warning-yellow so it stands out.
pub fn commit_sha(sha: &str) -> colored::ColoredString {
    sha.truecolor(210, 153, 34).bold() // #d29922 – GitHub warning yellow
}

// ── File path ─────────────────────────────────────────────────────────────────

/// Colours a file path in bright cyan.
pub fn file_path(path: &str) -> colored::ColoredString {
    path.cyan()
}

// ── Ownership bar ─────────────────────────────────────────────────────────────

/// Returns a coloured `"author (XX%)"` string; ≥80 % is red (silo risk),
/// ≥60 % is yellow (watch), otherwise green.
pub fn ownership_colored(author: &str, share: f64) -> String {
    let pct = (share * 100.0).round() as u32;
    let text = format!("{author} ({pct}%)");
    if pct >= 80 {
        text.red().bold().to_string()
    } else if pct >= 60 {
        text.yellow().to_string()
    } else {
        text.green().to_string()
    }
}

// ── Ranked list prefix ────────────────────────────────────────────────────────

/// Returns a left-padded rank string like `" 1."` in dim white.
pub fn rank_prefix(n: usize) -> colored::ColoredString {
    format!("{n:>2}.").bright_black()
}

// ── Suggestion / finding bullets ─────────────────────────────────────────────

/// Prints findings list for a given section, indented and prefixed with ⚠.
pub fn print_findings(findings: &[String], limit: usize) {
    for finding in findings.iter().take(limit) {
        println!("    {} {}", "⚠".yellow(), finding.bright_black());
    }
}

/// Prints a single suggestion line indented and prefixed with →.
pub fn print_suggestion(suggestion: &str) {
    println!("    {} {}", "→".bright_cyan(), suggestion.italic());
}

/// Prints a list of recommended next steps.
pub fn print_next_steps(suggestions: &[String]) {
    if suggestions.is_empty() {
        return;
    }
    println!();
    println!("  {} Recommended next steps", "★".yellow().bold());
    for s in suggestions {
        println!("    {} {}", "•".bright_black(), s);
    }
}
