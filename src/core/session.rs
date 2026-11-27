use std::fmt;
use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};

use crate::core::TabInfo;
use crate::core::storage::{StorageError, default_db_path, unix_timestamp_secs, with_db};

/// Default session name used by `:w`, `:wq`, and automatic startup restore.
pub const DEFAULT_SESSION_NAME: &str = "default";

/// Persisted representation of a single saved browser tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedTab {
    pub url: String,
    pub title: String,
}

/// Named browser session storing open tabs and the active tab index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub name: String,
    pub active_index: usize,
    pub tabs: Vec<SavedTab>,
}

impl Session {
    pub fn from_tabs(name: impl Into<String>, tabs: &[TabInfo], active_index: usize) -> Self {
        let saved_tabs: Vec<SavedTab> = tabs
            .iter()
            .map(|tab| SavedTab {
                url: tab.url.clone(),
                title: tab.title.clone(),
            })
            .collect();
        let clamped_active = if saved_tabs.is_empty() {
            0
        } else {
            active_index.min(saved_tabs.len() - 1)
        };
        Self {
            name: name.into(),
            active_index: clamped_active,
            tabs: saved_tabs,
        }
    }
}

/// Errors produced when validating, saving, or loading sessions.
#[derive(Debug)]
pub enum SessionError {
    InvalidName(String),
    NotFound(String),
    Io(std::io::Error),
    Db(rusqlite::Error),
}

impl From<StorageError> for SessionError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::Io(e) => Self::Io(e),
            StorageError::Db(e) => Self::Db(e),
        }
    }
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(name) => write!(
                f,
                "Invalid session name '{name}' (use letters, digits, '-' or '_')"
            ),
            Self::NotFound(name) => write!(f, "Session '{name}' not found"),
            Self::Io(err) => write!(f, "Session I/O error: {err}"),
            Self::Db(err) => write!(f, "Session database error: {err}"),
        }
    }
}

impl std::error::Error for SessionError {}

/// Validate a session name (`[A-Za-z0-9_-]+`).
pub fn validate_session_name(name: &str) -> Result<(), SessionError> {
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if valid {
        Ok(())
    } else {
        Err(SessionError::InvalidName(name.to_string()))
    }
}

/// SQLite-backed store for named browser sessions.
#[derive(Debug, Clone)]
pub struct SessionStore {
    db_path: PathBuf,
}

impl SessionStore {
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

    pub fn save(&self, session: &Session) -> Result<(), SessionError> {
        validate_session_name(&session.name)?;
        with_db(&self.db_path, |conn| {
            let tx = conn.transaction().map_err(SessionError::Db)?;

            let active_idx = i64::try_from(session.active_index).unwrap_or(0);
            tx.execute(
                "INSERT INTO sessions (name, active_index, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(name) DO UPDATE SET active_index = excluded.active_index,
                                                 updated_at = excluded.updated_at",
                params![session.name, active_idx, unix_timestamp_secs()],
            )
            .map_err(SessionError::Db)?;

            tx.execute(
                "DELETE FROM session_tabs WHERE session_name = ?1",
                params![session.name],
            )
            .map_err(SessionError::Db)?;

            {
                let mut stmt = tx
                    .prepare(
                        "INSERT INTO session_tabs (session_name, position, url, title)
                         VALUES (?1, ?2, ?3, ?4)",
                    )
                    .map_err(SessionError::Db)?;
                for (idx, tab) in session.tabs.iter().enumerate() {
                    let pos = i64::try_from(idx).unwrap_or(0);
                    stmt.execute(params![session.name, pos, tab.url, tab.title])
                        .map_err(SessionError::Db)?;
                }
            }

            tx.commit().map_err(SessionError::Db)?;
            Ok(())
        })
    }

    pub fn load(&self, name: &str) -> Result<Session, SessionError> {
        validate_session_name(name)?;
        with_db(&self.db_path, |conn| {
            let active_raw: Option<i64> = conn
                .query_row(
                    "SELECT active_index FROM sessions WHERE name = ?1",
                    params![name],
                    |row| row.get(0),
                )
                .optional()
                .map_err(SessionError::Db)?;

            let Some(active_raw) = active_raw else {
                return Err(SessionError::NotFound(name.to_string()));
            };

            let mut stmt = conn
                .prepare(
                    "SELECT url, title FROM session_tabs
                     WHERE session_name = ?1
                     ORDER BY position ASC",
                )
                .map_err(SessionError::Db)?;

            let tabs: Vec<SavedTab> = stmt
                .query_map(params![name], |row| {
                    Ok(SavedTab {
                        url: row.get(0)?,
                        title: row.get(1)?,
                    })
                })
                .map_err(SessionError::Db)?
                .collect::<Result<_, _>>()
                .map_err(SessionError::Db)?;

            let active_index = usize::try_from(active_raw).unwrap_or(0);
            let clamped_active = if tabs.is_empty() {
                0
            } else {
                active_index.min(tabs.len() - 1)
            };

            Ok(Session {
                name: name.to_string(),
                active_index: clamped_active,
                tabs,
            })
        })
    }

    /// Load a session if it exists, returning `Ok(None)` when missing.
    pub fn load_optional(&self, name: &str) -> Result<Option<Session>, SessionError> {
        match self.load(name) {
            Ok(session) => Ok(Some(session)),
            Err(SessionError::NotFound(_)) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Return all saved session names sorted alphabetically.
    pub fn list_names(&self) -> Result<Vec<String>, SessionError> {
        with_db(&self.db_path, |conn| {
            let mut stmt = conn
                .prepare("SELECT name FROM sessions ORDER BY name ASC")
                .map_err(SessionError::Db)?;
            let names = stmt
                .query_map([], |row| row.get(0))
                .map_err(SessionError::Db)?
                .collect::<Result<_, _>>()
                .map_err(SessionError::Db)?;
            Ok(names)
        })
    }
}
