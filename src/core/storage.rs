use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

/// Unified SQLite database filename stored inside `$XDG_CONFIG_HOME/nservo/`.
pub const DB_FILENAME: &str = "nservo.db";

/// Optional custom start-page HTML filename stored inside `$XDG_CONFIG_HOME/nservo/`.
pub const STARTPAGE_FILENAME: &str = "startpage.html";

/// Subdirectory inside `$XDG_CONFIG_HOME/nservo/` holding ABP-like adblock filter lists.
pub const ADBLOCK_DIRNAME: &str = "adblock";

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
    name TEXT PRIMARY KEY,
    active_index INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS session_tabs (
    session_name TEXT NOT NULL,
    position INTEGER NOT NULL,
    url TEXT NOT NULL,
    title TEXT NOT NULL,
    PRIMARY KEY (session_name, position),
    FOREIGN KEY (session_name) REFERENCES sessions(name) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL,
    title TEXT NOT NULL,
    visited_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_history_visited_at ON history(visited_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_history_url ON history(url);
"#;

/// Resolve the `nservo` config directory from `$XDG_CONFIG_HOME/nservo`
/// (falling back to `$HOME/.config/nservo`).
pub fn resolve_config_dir(xdg_config_home: Option<&str>, home: Option<&str>) -> PathBuf {
    let base = xdg_config_home
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            home.filter(|value| !value.trim().is_empty())
                .map(|dir| PathBuf::from(dir).join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from(".config"));
    base.join("nservo")
}

/// Resolve the SQLite database path under `$XDG_CONFIG_HOME/nservo/nservo.db`
/// (falling back to `$HOME/.config/nservo/nservo.db`).
pub fn resolve_db_path(xdg_config_home: Option<&str>, home: Option<&str>) -> PathBuf {
    resolve_config_dir(xdg_config_home, home).join(DB_FILENAME)
}

/// Default SQLite database path resolved from process environment variables.
pub fn default_db_path() -> PathBuf {
    let xdg = env::var("XDG_CONFIG_HOME").ok();
    let home = env::var("HOME").ok();
    resolve_db_path(xdg.as_deref(), home.as_deref())
}

/// Resolve the optional custom start page path under `$XDG_CONFIG_HOME/nservo/startpage.html`
/// (falling back to `$HOME/.config/nservo/startpage.html`).
pub fn resolve_startpage_path(xdg_config_home: Option<&str>, home: Option<&str>) -> PathBuf {
    resolve_config_dir(xdg_config_home, home).join(STARTPAGE_FILENAME)
}

/// Default custom start page path resolved from process environment variables.
pub fn default_startpage_path() -> PathBuf {
    let xdg = env::var("XDG_CONFIG_HOME").ok();
    let home = env::var("HOME").ok();
    resolve_startpage_path(xdg.as_deref(), home.as_deref())
}

/// Resolve the adblock filter-list directory under `$XDG_CONFIG_HOME/nservo/adblock`
/// (falling back to `$HOME/.config/nservo/adblock`).
pub fn resolve_adblock_dir(xdg_config_home: Option<&str>, home: Option<&str>) -> PathBuf {
    resolve_config_dir(xdg_config_home, home).join(ADBLOCK_DIRNAME)
}

/// Default adblock filter-list directory resolved from process environment variables.
pub fn default_adblock_dir() -> PathBuf {
    let xdg = env::var("XDG_CONFIG_HOME").ok();
    let home = env::var("HOME").ok();
    resolve_adblock_dir(xdg.as_deref(), home.as_deref())
}

/// Open (creating parent directories if needed) and initialize the SQLite database.
pub fn open_db(path: &Path) -> Result<Connection, StorageError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(StorageError::Io)?;
    }
    let conn = Connection::open(path).map_err(StorageError::Db)?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;\
         PRAGMA journal_mode = WAL;\
         PRAGMA synchronous = NORMAL;\
         PRAGMA busy_timeout = 3000;",
    )
    .map_err(StorageError::Db)?;
    conn.execute_batch(SCHEMA_SQL).map_err(StorageError::Db)?;
    Ok(conn)
}

fn db_pool() -> &'static Mutex<HashMap<PathBuf, Connection>> {
    static POOL: OnceLock<Mutex<HashMap<PathBuf, Connection>>> = OnceLock::new();
    POOL.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Run `f` against a cached persistent SQLite connection for `path`, avoiding
/// repeated file opens, schema DDL checks, and PRAGMA setup on every query.
pub fn with_db<T, E>(
    path: &Path,
    f: impl FnOnce(&mut Connection) -> Result<T, E>,
) -> Result<T, E>
where
    E: From<StorageError>,
{
    let mut pool = db_pool()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if pool.contains_key(path) && !path.exists() {
        pool.remove(path);
    }
    if !pool.contains_key(path) {
        let conn = open_db(path)?;
        pool.insert(path.to_path_buf(), conn);
    }
    let conn = pool.get_mut(path).expect("connection inserted above");
    f(conn)
}

/// Current UNIX timestamp in seconds.
pub fn unix_timestamp_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Low-level error opening or initializing the SQLite database.
#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Db(rusqlite::Error),
}
