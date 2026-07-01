use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub raw_transcript: String,
    pub final_text: String,
    pub model_id: String,
    pub enhancement_provider: Option<String>,
    pub inserted: bool,
    pub target_app: Option<String>,
}

impl HistoryEntry {
    pub fn new(
        raw_transcript: impl Into<String>,
        final_text: impl Into<String>,
        model_id: impl Into<String>,
        enhancement_provider: Option<String>,
        inserted: bool,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            raw_transcript: raw_transcript.into(),
            final_text: final_text.into(),
            model_id: model_id.into(),
            enhancement_provider,
            inserted,
            target_app: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("failed to create history directory: {0}")]
    CreateDir(#[source] std::io::Error),
    #[error("failed to read history file: {0}")]
    Read(#[source] std::io::Error),
    #[error("failed to parse history file: {0}")]
    Parse(#[source] serde_json::Error),
    #[error("failed to serialize history file: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("failed to write history file: {0}")]
    Write(#[source] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct HistoryStore {
    path: PathBuf,
}

impl HistoryStore {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            path: data_dir.into().join("history.json"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn list(&self, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let raw = fs::read_to_string(&self.path).map_err(HistoryError::Read)?;
        let mut entries =
            serde_json::from_str::<Vec<HistoryEntry>>(&raw).map_err(HistoryError::Parse)?;
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        entries.truncate(limit);
        Ok(entries)
    }

    pub fn append(&self, entry: HistoryEntry, max_items: usize) -> Result<(), HistoryError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(HistoryError::CreateDir)?;
        }

        let mut entries = self.list(max_items.saturating_add(1))?;
        entries.insert(0, entry);
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        entries.truncate(max_items);

        let raw = serde_json::to_string_pretty(&entries).map_err(HistoryError::Serialize)?;
        fs::write(&self.path, raw).map_err(HistoryError::Write)
    }

    pub fn clear(&self) -> Result<(), HistoryError> {
        if self.path.exists() {
            fs::write(&self.path, "[]").map_err(HistoryError::Write)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_enforces_limit_newest_first() {
        let temp_dir =
            std::env::temp_dir().join(format!("fluidvoice-history-test-{}", uuid::Uuid::new_v4()));
        let store = HistoryStore::new(&temp_dir);

        store
            .append(HistoryEntry::new("one", "one", "m", None, false), 2)
            .unwrap();
        store
            .append(HistoryEntry::new("two", "two", "m", None, false), 2)
            .unwrap();
        store
            .append(HistoryEntry::new("three", "three", "m", None, false), 2)
            .unwrap();

        let entries = store.list(10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].final_text, "three");
        assert_eq!(entries[1].final_text, "two");
        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
