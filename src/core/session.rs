use std::fmt;
use std::fs;
use std::path::PathBuf;

use gtk::glib;
use serde::{Deserialize, Serialize};

use crate::core::TabInfo;

/// Default session name used by `:w`, `:wq`, and automatic startup restore.
pub const DEFAULT_SESSION_NAME: &str = "default";

/// Serializable representation of a single saved browser tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedTab {
    pub url: String,
    pub title: String,
}

/// Named browser session storing open tabs and the active tab index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    Format(serde_json::Error),
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
            Self::Format(err) => write!(f, "Corrupted session file: {err}"),
        }
    }
}

impl std::error::Error for SessionError {}

/// Validate a session name to prevent path traversal or invalid filenames.
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

/// Filesystem-backed store for named JSON session files.
#[derive(Debug, Clone)]
pub struct SessionStore {
    dir: PathBuf,
}

impl SessionStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Default XDG data directory: `$XDG_DATA_HOME/nservo/sessions`.
    pub fn default_store() -> Self {
        Self::new(glib::user_data_dir().join("nservo").join("sessions"))
    }

    pub fn save(&self, session: &Session) -> Result<(), SessionError> {
        validate_session_name(&session.name)?;
        fs::create_dir_all(&self.dir).map_err(SessionError::Io)?;
        let json = serde_json::to_string_pretty(session).map_err(SessionError::Format)?;
        fs::write(self.session_path(&session.name), json).map_err(SessionError::Io)
    }

    pub fn load(&self, name: &str) -> Result<Session, SessionError> {
        validate_session_name(name)?;
        let path = self.session_path(name);
        let raw = match fs::read_to_string(&path) {
            Ok(data) => data,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(SessionError::NotFound(name.to_string()));
            }
            Err(err) => return Err(SessionError::Io(err)),
        };
        serde_json::from_str(&raw).map_err(SessionError::Format)
    }

    /// Load a session if it exists, returning `Ok(None)` when missing.
    pub fn load_optional(&self, name: &str) -> Result<Option<Session>, SessionError> {
        match self.load(name) {
            Ok(session) => Ok(Some(session)),
            Err(SessionError::NotFound(_)) => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn session_path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.json"))
    }
}
