use serde::{Deserialize, Serialize};

/// Bookmark entry stored in bookmarks.db.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkEntry {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub tags: Vec<String>,
    pub created_at: i64,
}

impl BookmarkEntry {
    pub fn new(id: i64, url: String, title: String, tags: Vec<String>, created_at: i64) -> Self {
        Self {
            id,
            url,
            title,
            tags,
            created_at,
        }
    }
}
