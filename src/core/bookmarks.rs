use std::fmt;
use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};

use crate::core::storage::{StorageError, default_db_path, unix_timestamp_secs, with_db};

/// A single saved browser bookmark.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bookmark {
    pub url: String,
    pub title: String,
}

impl Bookmark {
    /// Human-readable label shown in statusline notices and the bookmarks page.
    pub fn display_title(&self) -> &str {
        if !self.title.trim().is_empty() {
            self.title.trim()
        } else {
            &self.url
        }
    }
}

/// Outcome of attempting to add a page to bookmarks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddBookmarkOutcome {
    Added(Bookmark),
    AlreadyExists(Bookmark),
}

/// Errors produced when reading, validating, or saving bookmarks.
#[derive(Debug)]
pub enum BookmarkError {
    InvalidUrl(String),
    Io(std::io::Error),
    Db(rusqlite::Error),
}

impl From<StorageError> for BookmarkError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::Io(e) => Self::Io(e),
            StorageError::Db(e) => Self::Db(e),
        }
    }
}

impl fmt::Display for BookmarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(url) if url.is_empty() => {
                write!(f, "Cannot bookmark an empty page")
            }
            Self::InvalidUrl(url) => write!(f, "Cannot bookmark internal page '{url}'"),
            Self::Io(err) => write!(f, "Bookmarks I/O error: {err}"),
            Self::Db(err) => write!(f, "Bookmarks database error: {err}"),
        }
    }
}

impl std::error::Error for BookmarkError {}

fn is_bookmarkable_url(url: &str) -> bool {
    !url.is_empty()
        && !url.eq_ignore_ascii_case("about:blank")
        && !url.eq_ignore_ascii_case("about:help")
        && !url.eq_ignore_ascii_case("about:bookmarks")
        && !url.eq_ignore_ascii_case("about:history")
}

/// SQLite-backed store for browser bookmarks.
#[derive(Debug, Clone)]
pub struct BookmarkStore {
    db_path: PathBuf,
}

impl BookmarkStore {
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

    /// Load all saved bookmarks in insertion order.
    pub fn list(&self) -> Result<Vec<Bookmark>, BookmarkError> {
        with_db(&self.db_path, |conn| {
            let mut stmt = conn
                .prepare("SELECT url, title FROM bookmarks ORDER BY id ASC")
                .map_err(BookmarkError::Db)?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(Bookmark {
                        url: row.get(0)?,
                        title: row.get(1)?,
                    })
                })
                .map_err(BookmarkError::Db)?
                .collect::<Result<_, _>>()
                .map_err(BookmarkError::Db)?;
            Ok(rows)
        })
    }

    /// Add `(url, title)` to bookmarks, avoiding duplicate URLs.
    pub fn add(&self, url: &str, title: &str) -> Result<AddBookmarkOutcome, BookmarkError> {
        let clean_url = url.trim();
        if !is_bookmarkable_url(clean_url) {
            return Err(BookmarkError::InvalidUrl(clean_url.to_string()));
        }
        let clean_title = title.trim();
        with_db(&self.db_path, |conn| {
            let existing: Option<(i64, String)> = conn
                .query_row(
                    "SELECT id, title FROM bookmarks WHERE url = ?1",
                    params![clean_url],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(BookmarkError::Db)?;

            if let Some((id, existing_title)) = existing {
                let final_title = if existing_title.trim().is_empty() && !clean_title.is_empty() {
                    conn.execute(
                        "UPDATE bookmarks SET title = ?1 WHERE id = ?2",
                        params![clean_title, id],
                    )
                    .map_err(BookmarkError::Db)?;
                    clean_title.to_string()
                } else {
                    existing_title
                };
                return Ok(AddBookmarkOutcome::AlreadyExists(Bookmark {
                    url: clean_url.to_string(),
                    title: final_title,
                }));
            }

            conn.execute(
                "INSERT INTO bookmarks (url, title, created_at) VALUES (?1, ?2, ?3)",
                params![clean_url, clean_title, unix_timestamp_secs()],
            )
            .map_err(BookmarkError::Db)?;

            Ok(AddBookmarkOutcome::Added(Bookmark {
                url: clean_url.to_string(),
                title: clean_title.to_string(),
            }))
        })
    }

    /// Remove a bookmark by 1-based list index (e.g. `"2"`) or exact URL.
    pub fn remove(&self, target: &str) -> Result<Option<Bookmark>, BookmarkError> {
        let key = target.trim();
        if key.is_empty() {
            return Ok(None);
        }
        with_db(&self.db_path, |conn| {
            let row: Option<(i64, String, String)> = if let Ok(index) = key.parse::<usize>() {
                let Some(offset) = index.checked_sub(1) else {
                    return Ok(None);
                };
                conn.query_row(
                    "SELECT id, url, title FROM bookmarks ORDER BY id ASC LIMIT 1 OFFSET ?1",
                    params![i64::try_from(offset).unwrap_or(i64::MAX)],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()
                .map_err(BookmarkError::Db)?
            } else {
                conn.query_row(
                    "SELECT id, url, title FROM bookmarks WHERE url = ?1",
                    params![key],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()
                .map_err(BookmarkError::Db)?
            };

            let Some((id, url, title)) = row else {
                return Ok(None);
            };
            conn.execute("DELETE FROM bookmarks WHERE id = ?1", params![id])
                .map_err(BookmarkError::Db)?;
            Ok(Some(Bookmark { url, title }))
        })
    }
}
