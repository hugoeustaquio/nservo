use std::fmt;
use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};

use crate::core::storage::{StorageError, default_db_path, unix_timestamp_secs, with_db};

/// Coalescing window (seconds) to avoid duplicate rows for the same consecutive URL visit.
const CONSECUTIVE_DEDUP_SECS: i64 = 60;

/// Single persisted browsing history visit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub visited_at: i64,
}

impl HistoryEntry {
    pub fn display_title(&self) -> &str {
        if !self.title.trim().is_empty() {
            self.title.trim()
        } else {
            &self.url
        }
    }
}

/// Signal payload emitted by tab listeners to sync history visits and titles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistorySync {
    Visit { url: String, title: String },
    Title { url: String, title: String },
}

/// Errors produced when recording, querying, or clearing browsing history.
#[derive(Debug)]
pub enum HistoryError {
    Io(std::io::Error),
    Db(rusqlite::Error),
}

impl From<StorageError> for HistoryError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::Io(e) => Self::Io(e),
            StorageError::Db(e) => Self::Db(e),
        }
    }
}

impl fmt::Display for HistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "History I/O error: {err}"),
            Self::Db(err) => write!(f, "History database error: {err}"),
        }
    }
}

impl std::error::Error for HistoryError {}

/// Returns `true` for navigable external/file URLs that should be stored in history.
pub fn is_recordable_history_url(url: &str) -> bool {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    !lower.starts_with("about:") && !lower.starts_with("data:")
}

/// SQLite-backed store for browsing history.
#[derive(Debug, Clone)]
pub struct HistoryStore {
    db_path: PathBuf,
}

impl HistoryStore {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    /// Default SQLite database under `$XDG_CONFIG_HOME/nservo/nservo.db`
    /// (falling back to `$HOME/.config/nservo/nservo.db`).
    pub fn default_store() -> Self {
        Self::new(default_db_path())
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Record a visit to `url`, coalescing consecutive identical visits within 60 seconds.
    pub fn record_visit(&self, url: &str, title: &str) -> Result<bool, HistoryError> {
        let clean_url = url.trim();
        if !is_recordable_history_url(clean_url) {
            return Ok(false);
        }
        let clean_title = title.trim();
        let now = unix_timestamp_secs();
        with_db(&self.db_path, |conn| {
            let latest: Option<(i64, String, String, i64)> = conn
                .query_row(
                    "SELECT id, url, title, visited_at FROM history ORDER BY id DESC LIMIT 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .optional()
                .map_err(HistoryError::Db)?;

            if let Some((id, last_url, last_title, last_ts)) = latest
                && last_url == clean_url
                && now.saturating_sub(last_ts) <= CONSECUTIVE_DEDUP_SECS
            {
                let next_title = if !clean_title.is_empty() {
                    clean_title
                } else {
                    last_title.as_str()
                };
                conn.execute(
                    "UPDATE history SET title = ?1, visited_at = ?2 WHERE id = ?3",
                    params![next_title, now, id],
                )
                .map_err(HistoryError::Db)?;
                return Ok(true);
            }

            conn.execute(
                "INSERT INTO history (url, title, visited_at) VALUES (?1, ?2, ?3)",
                params![clean_url, clean_title, now],
            )
            .map_err(HistoryError::Db)?;
            Ok(true)
        })
    }

    /// Update the title of the most recent history entry matching `url`.
    pub fn update_title(&self, url: &str, title: &str) -> Result<bool, HistoryError> {
        let clean_url = url.trim();
        let clean_title = title.trim();
        if !is_recordable_history_url(clean_url) || clean_title.is_empty() {
            return Ok(false);
        }
        with_db(&self.db_path, |conn| {
            let latest_id: Option<i64> = conn
                .query_row(
                    "SELECT id FROM history WHERE url = ?1 ORDER BY id DESC LIMIT 1",
                    params![clean_url],
                    |row| row.get(0),
                )
                .optional()
                .map_err(HistoryError::Db)?;

            let Some(id) = latest_id else {
                return Ok(false);
            };
            conn.execute(
                "UPDATE history SET title = ?1 WHERE id = ?2",
                params![clean_title, id],
            )
            .map_err(HistoryError::Db)?;
            Ok(true)
        })
    }

    /// Return up to `limit` most recent visits (newest first).
    pub fn recent(&self, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        with_db(&self.db_path, |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, url, title, visited_at FROM history
                     ORDER BY visited_at DESC, id DESC
                     LIMIT ?1",
                )
                .map_err(HistoryError::Db)?;
            let rows = stmt
                .query_map(params![i64::try_from(limit).unwrap_or(200)], map_entry)
                .map_err(HistoryError::Db)?
                .collect::<Result<_, _>>()
                .map_err(HistoryError::Db)?;
            Ok(rows)
        })
    }

    /// Search history entries by case-insensitive substring match against URL or title.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return self.recent(limit);
        }
        let pattern = format!("%{}%", escape_like(trimmed));
        with_db(&self.db_path, |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT id, url, title, visited_at FROM history
                     WHERE lower(url) LIKE lower(?1) ESCAPE '\\'
                        OR lower(title) LIKE lower(?1) ESCAPE '\\'
                     ORDER BY visited_at DESC, id DESC
                     LIMIT ?2",
                )
                .map_err(HistoryError::Db)?;
            let rows = stmt
                .query_map(
                    params![pattern, i64::try_from(limit).unwrap_or(200)],
                    map_entry,
                )
                .map_err(HistoryError::Db)?
                .collect::<Result<_, _>>()
                .map_err(HistoryError::Db)?;
            Ok(rows)
        })
    }

    /// Delete all browsing history entries, returning the number removed.
    pub fn clear(&self) -> Result<usize, HistoryError> {
        with_db(&self.db_path, |conn| {
            conn.execute("DELETE FROM history", [])
                .map_err(HistoryError::Db)
        })
    }
}

fn map_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        id: row.get(0)?,
        url: row.get(1)?,
        title: row.get(2)?,
        visited_at: row.get(3)?,
    })
}

fn escape_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}
