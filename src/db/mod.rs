use crate::error::AppResult;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEventRecord {
    pub timestamp: String,
    pub actor: String,
    pub action: String,
    pub target: String,
    pub outcome: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRecord {
    pub id: String,
    pub timestamp: String,
    pub commit: String,
    pub actor: String,
    pub rating: i32,
    pub feedback: String,
    pub tags: Vec<String>,
}

pub trait AppDatabase {
    fn append_audit_event(&self, event: &AuditEventRecord) -> AppResult<()>;
    fn list_audit_events(&self, limit: usize) -> AppResult<Vec<AuditEventRecord>>;
    fn prune_audit_events_before(&self, cutoff_rfc3339: &str) -> AppResult<usize>;
    fn append_feedback(&self, feedback: &FeedbackRecord) -> AppResult<()>;
    fn list_feedback(&self, limit: usize) -> AppResult<Vec<FeedbackRecord>>;
    fn list_all_feedback(&self) -> AppResult<Vec<FeedbackRecord>>;
}

pub enum Database {
    Json(JsonDatabase),
    Postgres(PostgresDatabase),
}

impl Database {
    pub fn open(config: &crate::config::AppConfig) -> AppResult<Self> {
        match config.database.backend {
            crate::config::DatabaseBackend::Json => Ok(Self::Json(JsonDatabase::new()?)),
            crate::config::DatabaseBackend::Postgres => Ok(Self::Postgres(PostgresDatabase::new(
                &config.database.postgres_url,
            )?)),
        }
    }
}

impl AppDatabase for Database {
    fn append_audit_event(&self, event: &AuditEventRecord) -> AppResult<()> {
        match self {
            Self::Json(db) => db.append_audit_event(event),
            Self::Postgres(db) => db.append_audit_event(event),
        }
    }

    fn list_audit_events(&self, limit: usize) -> AppResult<Vec<AuditEventRecord>> {
        match self {
            Self::Json(db) => db.list_audit_events(limit),
            Self::Postgres(db) => db.list_audit_events(limit),
        }
    }

    fn prune_audit_events_before(&self, cutoff_rfc3339: &str) -> AppResult<usize> {
        match self {
            Self::Json(db) => db.prune_audit_events_before(cutoff_rfc3339),
            Self::Postgres(db) => db.prune_audit_events_before(cutoff_rfc3339),
        }
    }

    fn append_feedback(&self, feedback: &FeedbackRecord) -> AppResult<()> {
        match self {
            Self::Json(db) => db.append_feedback(feedback),
            Self::Postgres(db) => db.append_feedback(feedback),
        }
    }

    fn list_feedback(&self, limit: usize) -> AppResult<Vec<FeedbackRecord>> {
        match self {
            Self::Json(db) => db.list_feedback(limit),
            Self::Postgres(db) => db.list_feedback(limit),
        }
    }

    fn list_all_feedback(&self) -> AppResult<Vec<FeedbackRecord>> {
        match self {
            Self::Json(db) => db.list_all_feedback(),
            Self::Postgres(db) => db.list_all_feedback(),
        }
    }
}

pub struct JsonDatabase {
    feedback_path: PathBuf,
    audit_path: PathBuf,
}

impl JsonDatabase {
    fn new() -> AppResult<Self> {
        let feedback_path = crate::storage::feedback_json_path()?;
        let audit_path = crate::storage::audit_json_path()?;
        if let Some(parent) = feedback_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(parent) = audit_path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self {
            feedback_path,
            audit_path,
        })
    }
}

impl AppDatabase for JsonDatabase {
    fn append_audit_event(&self, event: &AuditEventRecord) -> AppResult<()> {
        let mut events = read_json_vec::<AuditEventRecord>(&self.audit_path)?;
        events.push(event.clone());
        write_json_vec(&self.audit_path, &events)
    }

    fn list_audit_events(&self, limit: usize) -> AppResult<Vec<AuditEventRecord>> {
        let mut events = read_json_vec::<AuditEventRecord>(&self.audit_path)?;
        events.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
        if events.len() > limit {
            events.truncate(limit);
        }
        Ok(events)
    }

    fn prune_audit_events_before(&self, cutoff_rfc3339: &str) -> AppResult<usize> {
        let events = read_json_vec::<AuditEventRecord>(&self.audit_path)?;
        let original_len = events.len();
        let filtered = events
            .into_iter()
            .filter(|event| event.timestamp.as_str() >= cutoff_rfc3339)
            .collect::<Vec<_>>();
        let removed = original_len.saturating_sub(filtered.len());
        write_json_vec(&self.audit_path, &filtered)?;
        Ok(removed)
    }

    fn append_feedback(&self, feedback: &FeedbackRecord) -> AppResult<()> {
        let mut records = read_json_vec::<FeedbackRecord>(&self.feedback_path)?;
        records.push(feedback.clone());
        write_json_vec(&self.feedback_path, &records)
    }

    fn list_feedback(&self, limit: usize) -> AppResult<Vec<FeedbackRecord>> {
        let mut records = read_json_vec::<FeedbackRecord>(&self.feedback_path)?;
        records.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
        if records.len() > limit {
            records.truncate(limit);
        }
        Ok(records)
    }

    fn list_all_feedback(&self) -> AppResult<Vec<FeedbackRecord>> {
        let mut records = read_json_vec::<FeedbackRecord>(&self.feedback_path)?;
        records.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
        Ok(records)
    }
}

pub struct PostgresDatabase {
    url: String,
}

impl PostgresDatabase {
    fn new(url: &str) -> AppResult<Self> {
        if url.trim().is_empty() {
            return Err(crate::error::AppError::message(
                "Postgres backend requires `database.postgres_url` to be set.",
            ));
        }

        let db = Self {
            url: url.trim().to_string(),
        };
        db.init()?;
        Ok(db)
    }

    fn connect(&self) -> AppResult<postgres::Client> {
        Ok(postgres::Client::connect(&self.url, postgres::NoTls)?)
    }

    fn init(&self) -> AppResult<()> {
        let mut client = self.connect()?;
        client.batch_execute(
            "CREATE TABLE IF NOT EXISTS audit_events (
                id BIGSERIAL PRIMARY KEY,
                timestamp TIMESTAMPTZ NOT NULL,
                actor TEXT NOT NULL,
                action TEXT NOT NULL,
                target TEXT NOT NULL,
                outcome TEXT NOT NULL,
                metadata JSONB NOT NULL
            );
            CREATE TABLE IF NOT EXISTS feedback (
                id TEXT PRIMARY KEY,
                timestamp TIMESTAMPTZ NOT NULL,
                commit_hash TEXT NOT NULL,
                actor TEXT NOT NULL,
                rating INTEGER NOT NULL,
                feedback TEXT NOT NULL,
                tags JSONB NOT NULL
            );",
        )?;
        Ok(())
    }
}

impl AppDatabase for PostgresDatabase {
    fn append_audit_event(&self, event: &AuditEventRecord) -> AppResult<()> {
        let mut client = self.connect()?;
        client.execute(
            "INSERT INTO audit_events (timestamp, actor, action, target, outcome, metadata)
             VALUES ($1::text::timestamptz, $2, $3, $4, $5, $6::text::jsonb)",
            &[
                &event.timestamp,
                &event.actor,
                &event.action,
                &event.target,
                &event.outcome,
                &serde_json::to_string(&event.metadata)?,
            ],
        )?;
        Ok(())
    }

    fn list_audit_events(&self, limit: usize) -> AppResult<Vec<AuditEventRecord>> {
        let mut client = self.connect()?;
        let rows = client.query(
            "SELECT timestamp::text, actor, action, target, outcome, metadata::text
             FROM audit_events
             ORDER BY timestamp DESC
             LIMIT $1",
            &[&(limit as i64)],
        )?;
        Ok(rows
            .into_iter()
            .map(|row| AuditEventRecord {
                timestamp: row.get::<_, String>(0),
                actor: row.get::<_, String>(1),
                action: row.get::<_, String>(2),
                target: row.get::<_, String>(3),
                outcome: row.get::<_, String>(4),
                metadata: serde_json::from_str(&row.get::<_, String>(5))
                    .unwrap_or(serde_json::Value::Null),
            })
            .collect())
    }

    fn prune_audit_events_before(&self, cutoff_rfc3339: &str) -> AppResult<usize> {
        let mut client = self.connect()?;
        let removed = client.execute(
            "DELETE FROM audit_events WHERE timestamp < $1::text::timestamptz",
            &[&cutoff_rfc3339],
        )?;
        Ok(removed as usize)
    }

    fn append_feedback(&self, feedback: &FeedbackRecord) -> AppResult<()> {
        let mut client = self.connect()?;
        client.execute(
            "INSERT INTO feedback (id, timestamp, commit_hash, actor, rating, feedback, tags)
             VALUES ($1, $2::text::timestamptz, $3, $4, $5, $6, $7::text::jsonb)
             ON CONFLICT (id) DO UPDATE SET
               timestamp = EXCLUDED.timestamp,
               commit_hash = EXCLUDED.commit_hash,
               actor = EXCLUDED.actor,
               rating = EXCLUDED.rating,
               feedback = EXCLUDED.feedback,
               tags = EXCLUDED.tags",
            &[
                &feedback.id,
                &feedback.timestamp,
                &feedback.commit,
                &feedback.actor,
                &feedback.rating,
                &feedback.feedback,
                &serde_json::to_string(&feedback.tags)?,
            ],
        )?;
        Ok(())
    }

    fn list_feedback(&self, limit: usize) -> AppResult<Vec<FeedbackRecord>> {
        let mut client = self.connect()?;
        let rows = client.query(
            "SELECT id, timestamp::text, commit_hash, actor, rating, feedback, tags::text
             FROM feedback
             ORDER BY timestamp DESC
             LIMIT $1",
            &[&(limit as i64)],
        )?;
        Ok(rows
            .into_iter()
            .map(|row| FeedbackRecord {
                id: row.get::<_, String>(0),
                timestamp: row.get::<_, String>(1),
                commit: row.get::<_, String>(2),
                actor: row.get::<_, String>(3),
                rating: row.get::<_, i32>(4),
                feedback: row.get::<_, String>(5),
                tags: serde_json::from_str(&row.get::<_, String>(6)).unwrap_or_default(),
            })
            .collect())
    }

    fn list_all_feedback(&self) -> AppResult<Vec<FeedbackRecord>> {
        let mut client = self.connect()?;
        let rows = client.query(
            "SELECT id, timestamp::text, commit_hash, actor, rating, feedback, tags::text
             FROM feedback
             ORDER BY timestamp DESC",
            &[],
        )?;
        Ok(rows
            .into_iter()
            .map(|row| FeedbackRecord {
                id: row.get::<_, String>(0),
                timestamp: row.get::<_, String>(1),
                commit: row.get::<_, String>(2),
                actor: row.get::<_, String>(3),
                rating: row.get::<_, i32>(4),
                feedback: row.get::<_, String>(5),
                tags: serde_json::from_str(&row.get::<_, String>(6)).unwrap_or_default(),
            })
            .collect())
    }
}

fn read_json_vec<T: DeserializeOwned>(path: &Path) -> AppResult<Vec<T>> {
    match fs::read_to_string(path) {
        Ok(raw) => Ok(serde_json::from_str(&raw)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error.into()),
    }
}

fn write_json_vec<T: Serialize>(path: &Path, values: &[T]) -> AppResult<()> {
    let raw = serde_json::to_string_pretty(values)?;
    fs::write(path, raw)?;
    Ok(())
}
