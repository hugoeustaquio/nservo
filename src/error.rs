use thiserror::Error;

#[derive(Error, Debug)]
pub enum NServoError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Lua runtime error: {0}")]
    Lua(#[from] mlua::Error),

    #[error("Engine error: {0}")]
    Engine(String),

    #[error("Command syntax error: {0}")]
    CommandSyntax(String),

    #[error("Plugin error ({name}): {reason}")]
    Plugin { name: String, reason: String },

    #[error("Security violation: {0}")]
    Security(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, NServoError>;
