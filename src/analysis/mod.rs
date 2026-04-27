pub mod behavior_patterns;
pub mod bug_predictor;
pub mod codebase_insights;
pub mod diff_parser;
pub mod impact_analysis;
pub mod intent_detection;
pub mod knowledge_risk;
pub mod performance_analyzer;
pub mod quality_analyzer;
pub mod refactor_prioritizer;
pub mod security_analyzer;

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct CommitAnalysis {
    pub intent: IntentClassification,
    pub diff: DiffSummary,
    pub impact: ImpactSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct IntentClassification {
    pub category: ChangeCategory,
    pub urgency: UrgencyLevel,
    pub risk: RiskLevel,
    pub scope: ChangeScope,
    pub conventional_type: String,
    pub conventional_scope: String,
    pub breaking_change: bool,
    pub signals: Vec<String>,
    pub confidence: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct DiffSummary {
    pub files_changed: usize,
    pub files_added: usize,
    pub files_deleted: usize,
    pub files_renamed: usize,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub net_lines: isize,
    pub complexity_delta: isize,
    pub import_changes: Vec<ImportChange>,
    pub symbol_changes: Vec<SymbolChange>,
    pub file_stats: Vec<FileDiffStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct ImpactSummary {
    pub impact_score: u32,
    pub direct_dependents: Vec<String>,
    pub transitive_dependents: Vec<String>,
    pub circular_dependencies: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct FileDiffStat {
    pub path: String,
    pub previous_path: Option<String>,
    pub added: usize,
    pub removed: usize,
    pub operation: FileOperation,
    pub language: String,
    pub complexity_delta: isize,
    pub import_changes: Vec<ImportChange>,
    pub symbol_changes: Vec<SymbolChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct ImportChange {
    pub file_path: String,
    pub statement: String,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct SymbolChange {
    pub file_path: String,
    pub symbol_name: String,
    pub signature: String,
    pub kind: ChangeKind,
    pub symbol_kind: SymbolKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeCategory {
    BugFix,
    Feature,
    Refactor,
    Performance,
    Documentation,
    DependencyUpdate,
    Test,
    Chore,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum UrgencyLevel {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeScope {
    SingleFile,
    CrossFile,
    Broad,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum RiskLevel {
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum FileOperation {
    Added,
    Deleted,
    Renamed,
    Copied,
    #[default]
    Modified,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeKind {
    Added,
    Removed,
    #[default]
    Modified,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum SymbolKind {
    Function,
    Type,
    Module,
    #[default]
    Unknown,
}

impl CommitAnalysis {
    pub fn is_empty(&self) -> bool {
        self.intent.category == ChangeCategory::Unknown
            && self.diff.files_changed == 0
            && self.impact.impact_score == 0
    }
}

impl IntentClassification {
    pub fn summary(&self) -> String {
        let mut parts = vec![
            self.category.to_string(),
            format!("{} urgency", self.urgency),
            format!("{} risk", self.risk),
            format!("{} scope", self.scope),
        ];

        if !self.conventional_type.is_empty() {
            if self.conventional_scope.is_empty() {
                parts.push(format!("conventional `{}`", self.conventional_type));
            } else {
                parts.push(format!(
                    "conventional `{}({})`",
                    self.conventional_type, self.conventional_scope
                ));
            }
        }

        if self.breaking_change {
            parts.push("breaking-change".to_string());
        }

        parts.push(format!("{}% confidence", self.confidence));
        parts.join(" | ")
    }

    pub fn signals_summary(&self, limit: usize) -> Option<String> {
        if self.signals.is_empty() {
            None
        } else {
            Some(
                self.signals
                    .iter()
                    .take(limit)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        }
    }
}

impl DiffSummary {
    pub fn summary(&self) -> Option<String> {
        if self.files_changed == 0 {
            None
        } else {
            let mut parts = vec![format!(
                "{} files changed, +{} / -{} (net {:+})",
                self.files_changed, self.lines_added, self.lines_removed, self.net_lines
            )];

            let operation_summary = self.file_operation_summary();
            if !operation_summary.is_empty() {
                parts.push(operation_summary);
            }

            Some(parts.join("; "))
        }
    }

    pub fn top_files_summary(&self, limit: usize) -> Option<String> {
        if self.file_stats.is_empty() {
            return None;
        }

        let summary = self
            .file_stats
            .iter()
            .take(limit)
            .map(|stat| format!("{} (+{} -{})", stat.path, stat.added, stat.removed))
            .collect::<Vec<_>>()
            .join(", ");

        Some(summary)
    }

    pub fn semantic_summary(&self) -> Option<String> {
        let mut parts = Vec::new();

        if !self.import_changes.is_empty() {
            let imports_added = self
                .import_changes
                .iter()
                .filter(|change| change.kind == ChangeKind::Added)
                .count();
            let imports_removed = self
                .import_changes
                .iter()
                .filter(|change| change.kind == ChangeKind::Removed)
                .count();
            parts.push(format!("imports +{} / -{}", imports_added, imports_removed));
        }

        if !self.symbol_changes.is_empty() {
            parts.push(format!("{} symbols touched", self.symbol_changes.len()));
        }

        if self.complexity_delta != 0 {
            parts.push(format!("complexity {:+}", self.complexity_delta));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("; "))
        }
    }

    pub fn changed_symbols_summary(&self, limit: usize) -> Option<String> {
        if self.symbol_changes.is_empty() {
            return None;
        }

        let summary = self
            .symbol_changes
            .iter()
            .take(limit)
            .map(|change| {
                format!(
                    "{} {} ({})",
                    change.kind, change.symbol_name, change.symbol_kind
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        Some(summary)
    }

    pub fn import_summary(&self, limit: usize) -> Option<String> {
        if self.import_changes.is_empty() {
            return None;
        }

        let summary = self
            .import_changes
            .iter()
            .take(limit)
            .map(|change| format!("{} {}", change.kind, change.statement))
            .collect::<Vec<_>>()
            .join(", ");

        Some(summary)
    }

    fn file_operation_summary(&self) -> String {
        let mut parts = Vec::new();

        if self.files_added > 0 {
            parts.push(format!("{} added", self.files_added));
        }
        if self.files_deleted > 0 {
            parts.push(format!("{} deleted", self.files_deleted));
        }
        if self.files_renamed > 0 {
            parts.push(format!("{} renamed", self.files_renamed));
        }

        parts.join(", ")
    }
}

impl ImpactSummary {
    pub fn summary(&self) -> Option<String> {
        if self.impact_score == 0
            && self.direct_dependents.is_empty()
            && self.transitive_dependents.is_empty()
        {
            None
        } else {
            Some(format!(
                "impact score {} | {} direct | {} transitive",
                self.impact_score,
                self.direct_dependents.len(),
                self.transitive_dependents.len()
            ))
        }
    }

    pub fn top_direct_summary(&self, limit: usize) -> Option<String> {
        summarize_paths(&self.direct_dependents, limit)
    }

    pub fn top_transitive_summary(&self, limit: usize) -> Option<String> {
        summarize_paths(&self.transitive_dependents, limit)
    }

    pub fn circular_summary(&self, limit: usize) -> Option<String> {
        if self.circular_dependencies.is_empty() {
            return None;
        }

        let summary = self
            .circular_dependencies
            .iter()
            .take(limit)
            .map(|cycle| cycle.join(" -> "))
            .collect::<Vec<_>>()
            .join("; ");

        Some(summary)
    }
}

impl Display for ChangeCategory {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::BugFix => "bug-fix",
            Self::Feature => "feature",
            Self::Refactor => "refactor",
            Self::Performance => "performance",
            Self::Documentation => "documentation",
            Self::DependencyUpdate => "dependency-update",
            Self::Test => "test",
            Self::Chore => "chore",
            Self::Unknown => "unknown",
        };

        write!(formatter, "{label}")
    }
}

impl Display for UrgencyLevel {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Critical => "critical",
        };

        write!(formatter, "{label}")
    }
}

impl Display for ChangeScope {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::SingleFile => "single-file",
            Self::CrossFile => "cross-file",
            Self::Broad => "broad",
            Self::Unknown => "unknown",
        };

        write!(formatter, "{label}")
    }
}

impl Display for RiskLevel {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        };

        write!(formatter, "{label}")
    }
}

impl Display for FileOperation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Added => "added",
            Self::Deleted => "deleted",
            Self::Renamed => "renamed",
            Self::Copied => "copied",
            Self::Modified => "modified",
        };

        write!(formatter, "{label}")
    }
}

impl Display for ChangeKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Modified => "modified",
        };

        write!(formatter, "{label}")
    }
}

impl Display for SymbolKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Function => "function",
            Self::Type => "type",
            Self::Module => "module",
            Self::Unknown => "symbol",
        };

        write!(formatter, "{label}")
    }
}

fn summarize_paths(paths: &[String], limit: usize) -> Option<String> {
    if paths.is_empty() {
        None
    } else {
        Some(
            paths
                .iter()
                .take(limit)
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        )
    }
}
