use crate::db::{AppDatabase, AuditEventRecord, Database};
use crate::error::AppResult;

pub fn record(
    config: &crate::config::AppConfig,
    actor: &str,
    action: &str,
    target: &str,
    outcome: &str,
    metadata: serde_json::Value,
) -> AppResult<()> {
    if !config.audit.enabled {
        return Ok(());
    }

    let db = Database::open(config)?;
    let event = AuditEventRecord {
        timestamp: chrono::Utc::now().to_rfc3339(),
        actor: actor.to_string(),
        action: action.to_string(),
        target: target.to_string(),
        outcome: outcome.to_string(),
        metadata,
    };
    db.append_audit_event(&event)
}

pub fn recent(config: &crate::config::AppConfig, limit: usize) -> AppResult<Vec<AuditEventRecord>> {
    let db = Database::open(config)?;
    db.list_audit_events(limit.max(1))
}

pub fn prune(config: &crate::config::AppConfig, days: u32) -> AppResult<usize> {
    let db = Database::open(config)?;
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
    db.prune_audit_events_before(&cutoff.to_rfc3339())
}
