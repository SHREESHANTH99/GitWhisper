use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::sync::{Mutex, OnceLock};

const MEMORY_CACHE_LIMIT: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationCacheRecord {
    pub id: String,
    pub commit_hash: String,
    pub file_path: String,
    pub explanation: String,
    pub metadata: Value,
    pub source: String,
    pub ai_model: String,
    pub commit_history_hash: String,
    pub created_at: String,
    #[serde(default)]
    pub last_accessed: String,
    #[serde(default)]
    pub access_count: u64,
}

#[derive(Default)]
struct MemoryCache {
    order: VecDeque<String>,
    entries: HashMap<String, ExplanationCacheRecord>,
}

static MEMORY_CACHE: OnceLock<Mutex<MemoryCache>> = OnceLock::new();

pub fn cache_key(file_path: &str, commit_hash: &str, history_hash: &str, ai_model: &str) -> String {
    format!("{file_path}::{commit_hash}::{history_hash}::{ai_model}")
}

pub fn get_explanation(id: &str) -> Option<ExplanationCacheRecord> {
    if let Some(record) = get_from_memory(id) {
        update_index_access(id);
        return Some(record);
    }

    let mut index = load_index()?;
    let record = index.get_mut(id)?;
    record.access_count += 1;
    record.last_accessed = chrono::Utc::now().to_rfc3339();
    let cloned = record.clone();
    save_index(&index);
    store_in_memory(cloned.clone());
    Some(cloned)
}

pub fn put_explanation(record: &ExplanationCacheRecord) {
    let mut index = load_index().unwrap_or_default();
    let mut stored = record.clone();
    stored.last_accessed = stored.created_at.clone();
    index.insert(stored.id.clone(), stored.clone());
    save_index(&index);
    store_in_memory(stored);
}

fn get_from_memory(id: &str) -> Option<ExplanationCacheRecord> {
    let cache = MEMORY_CACHE.get_or_init(|| Mutex::new(MemoryCache::default()));
    let cache = cache.lock().ok()?;
    cache.entries.get(id).cloned()
}

fn store_in_memory(record: ExplanationCacheRecord) {
    let cache = MEMORY_CACHE.get_or_init(|| Mutex::new(MemoryCache::default()));
    let Ok(mut cache) = cache.lock() else {
        return;
    };

    if !cache.entries.contains_key(&record.id) {
        cache.order.push_back(record.id.clone());
    }
    cache.entries.insert(record.id.clone(), record);

    while cache.order.len() > MEMORY_CACHE_LIMIT {
        if let Some(oldest) = cache.order.pop_front() {
            cache.entries.remove(&oldest);
        }
    }
}

fn update_index_access(id: &str) {
    let Some(mut index) = load_index() else {
        return;
    };
    let Some(record) = index.get_mut(id) else {
        return;
    };
    record.access_count += 1;
    record.last_accessed = chrono::Utc::now().to_rfc3339();
    save_index(&index);
}

fn load_index() -> Option<HashMap<String, ExplanationCacheRecord>> {
    let path = cache_index_path()?;
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<HashMap<String, ExplanationCacheRecord>>(&raw).ok(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(HashMap::new()),
        Err(_) => None,
    }
}

fn save_index(index: &HashMap<String, ExplanationCacheRecord>) {
    let Some(path) = cache_index_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let Ok(raw) = serde_json::to_string_pretty(index) else {
        return;
    };
    let _ = fs::write(path, raw);
}

fn cache_index_path() -> Option<std::path::PathBuf> {
    crate::storage::cache_dir()
        .ok()
        .map(|directory| directory.join("cache-index.json"))
}
