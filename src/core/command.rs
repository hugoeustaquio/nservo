use std::fmt;

use crate::core::BrowserAction;
use crate::core::session::{DEFAULT_SESSION_NAME, validate_session_name};
pub use crate::core::url_norm::normalize_url;
use crate::core::url_norm::starts_with_ascii_case;

/// Errors produced when parsing a Neovim-style command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    Empty,
    MissingArgument(&'static str),
    InvalidArgument(&'static str),
    UnknownCommand(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "Empty command"),
            Self::MissingArgument(usage) => write!(f, "Missing argument — usage: {usage}"),
            Self::InvalidArgument(msg) => write!(f, "Invalid argument: {msg}"),
            Self::UnknownCommand(cmd) => write!(f, "Not an editor command: :{cmd}"),
        }
    }
}

impl std::error::Error for CommandError {}

/// Parse an Ex-style command string (with or without a leading `:`).
///
/// Notable case-sensitive commands:
/// - `:open <URL>` (or `:o <URL>`): opens `URL` in the current tab.
/// - `:Open <URL>` (or `:O <URL>`, `:tabopen <URL>`): opens `URL` in a new tab.
pub fn parse_command(raw: &str) -> Result<BrowserAction, CommandError> {
    let trimmed = raw.trim().strip_prefix(':').unwrap_or(raw.trim()).trim();
    if trimmed.is_empty() {
        return Err(CommandError::Empty);
    }

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let name = parts.next().unwrap_or("");
    let rest = parts.next().map(str::trim).unwrap_or("");

    match name {
        "open" | "o" => parse_url_arg(rest, ":open <URL>").map(BrowserAction::OpenInCurrentTab),
        "Open" | "O" | "tabopen" => {
            parse_url_arg(rest, ":Open <URL>").map(BrowserAction::OpenInNewTab)
        }
        "tabnew" => {
            if rest.is_empty() {
                Ok(BrowserAction::OpenStartPageInNewTab)
            } else {
                Ok(BrowserAction::OpenInNewTab(normalize_url(rest)))
            }
        }
        "tabnext" | "tabn" => Ok(BrowserAction::NextTab),
        "tabprev" | "tabp" | "tabprevious" => Ok(BrowserAction::PrevTab),
        "tabclose" | "tabc" | "tc" | "bd" | "close" => Ok(BrowserAction::CloseTab),
        "reload" | "r" => Ok(BrowserAction::Reload),
        "back" => Ok(BrowserAction::GoBack),
        "forward" => Ok(BrowserAction::GoForward),
        "zoom" => parse_zoom_arg(rest),
        "inspect" | "inspector" | "devtools" => Ok(BrowserAction::ToggleInspector),
        "noh" | "nohlsearch" => Ok(BrowserAction::ClearSearch),
        "help" | "h" => Ok(BrowserAction::ShowHelp),
        "config" => Ok(BrowserAction::ShowConfig),
        "w" | "write" => parse_optional_session(rest).map(BrowserAction::SaveSession),
        "wq" => parse_optional_session(rest).map(BrowserAction::SaveSessionAndQuit),
        "session" => parse_required_session(rest).map(BrowserAction::LoadSession),
        "bookmark" | "bm-add" => Ok(BrowserAction::AddBookmark),
        "bookmarks" | "bm" => Ok(BrowserAction::ShowBookmarks),
        "bookmark-del" | "bmdel" => Ok(BrowserAction::RemoveBookmark(
            (!rest.is_empty()).then(|| rest.to_string()),
        )),
        "history" | "hist" => Ok(BrowserAction::ShowHistory(
            (!rest.is_empty()).then(|| rest.to_string()),
        )),
        "history-clear" | "histclear" => Ok(BrowserAction::ClearHistory),
        "adblock" => parse_adblock_url_arg(rest).map(BrowserAction::AddAdblockList),
        "adblock-update" => Ok(BrowserAction::UpdateAdblockLists),
        "plugin" | "plug" => parse_plugin_run_arg(rest),
        "plugin-install" => parse_nonempty_arg(rest, ":plugin-install <owner/repo|git-url>")
            .map(BrowserAction::InstallPlugin),
        "plugin-update" => Ok(BrowserAction::UpdatePlugins(
            (!rest.is_empty()).then(|| rest.to_string()),
        )),
        "plugin-remove" => {
            parse_nonempty_arg(rest, ":plugin-remove <name>").map(BrowserAction::RemovePlugin)
        }
        "q" | "quit" | "qa" => Ok(BrowserAction::Quit),
        other => Err(CommandError::UnknownCommand(other.to_string())),
    }
}

fn parse_optional_session(arg: &str) -> Result<String, CommandError> {
    let name = if arg.is_empty() {
        DEFAULT_SESSION_NAME
    } else {
        arg
    };
    validate_session_name(name).map_err(|_| {
        CommandError::InvalidArgument("session name must use only letters, digits, '-' or '_'")
    })?;
    Ok(name.to_string())
}

fn parse_required_session(arg: &str) -> Result<String, CommandError> {
    if arg.is_empty() {
        return Err(CommandError::MissingArgument(":session <name>"));
    }
    validate_session_name(arg).map_err(|_| {
        CommandError::InvalidArgument("session name must use only letters, digits, '-' or '_'")
    })?;
    Ok(arg.to_string())
}

fn parse_url_arg(arg: &str, usage: &'static str) -> Result<String, CommandError> {
    if arg.is_empty() {
        return Err(CommandError::MissingArgument(usage));
    }
    Ok(normalize_url(arg))
}

fn parse_nonempty_arg(arg: &str, usage: &'static str) -> Result<String, CommandError> {
    if arg.is_empty() {
        return Err(CommandError::MissingArgument(usage));
    }
    Ok(arg.to_string())
}

fn parse_plugin_run_arg(arg: &str) -> Result<BrowserAction, CommandError> {
    if arg.is_empty() {
        return Err(CommandError::MissingArgument(":plugin <name> [args]"));
    }
    let mut parts = arg.splitn(2, char::is_whitespace);
    let name = parts.next().unwrap_or("").trim().to_string();
    let args = parts.next().map(str::trim).unwrap_or("").to_string();
    Ok(BrowserAction::RunPlugin { name, args })
}

fn parse_adblock_url_arg(arg: &str) -> Result<String, CommandError> {
    if arg.is_empty() {
        return Err(CommandError::MissingArgument(":adblock <URL>"));
    }
    let normalized = normalize_url(arg);
    if !starts_with_ascii_case(&normalized, "http://")
        && !starts_with_ascii_case(&normalized, "https://")
    {
        return Err(CommandError::InvalidArgument(
            "adblock URL must use http:// or https://",
        ));
    }
    if arg.chars().any(char::is_whitespace) {
        return Err(CommandError::InvalidArgument(
            "adblock URL must not contain spaces",
        ));
    }
    Ok(normalized)
}

fn parse_zoom_arg(arg: &str) -> Result<BrowserAction, CommandError> {
    if arg.is_empty() {
        return Err(CommandError::MissingArgument(
            ":zoom <factor|reset> (e.g. :zoom 1.25)",
        ));
    }
    if arg.eq_ignore_ascii_case("reset") {
        return Ok(BrowserAction::SetZoom(1.0));
    }
    let factor: f64 = arg
        .parse()
        .map_err(|_| CommandError::InvalidArgument("zoom must be a number or 'reset'"))?;
    if !factor.is_finite() || !(0.1..=10.0).contains(&factor) {
        return Err(CommandError::InvalidArgument(
            "zoom must be between 0.1 and 10.0",
        ));
    }
    Ok(BrowserAction::SetZoom(factor))
}
