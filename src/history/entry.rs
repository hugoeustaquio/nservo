use serde::{Deserialize, Serialize};

/// Record of a visited URL in the browser history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub timestamp: i64,
    pub visit_count: u32,
}

impl HistoryEntry {
    pub fn new(id: i64, url: String, title: String, timestamp: i64, visit_count: u32) -> Self {
        Self {
            id,
            url,
            title,
            timestamp,
            visit_count,
        }
    }
}
