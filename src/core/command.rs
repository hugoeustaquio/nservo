use std::fmt::{self, Write};
use std::net::{Ipv4Addr, Ipv6Addr};

use crate::core::BrowserAction;
use crate::core::session::{DEFAULT_SESSION_NAME, validate_session_name};

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
        "Open" | "O" | "tabopen" | "tabnew" => {
            parse_url_arg(rest, ":Open <URL>").map(BrowserAction::OpenInNewTab)
        }
        "tabnext" | "tabn" => Ok(BrowserAction::NextTab),
        "tabprev" | "tabp" | "tabprevious" => Ok(BrowserAction::PrevTab),
        "tabclose" | "tabc" | "tc" | "bd" | "close" => Ok(BrowserAction::CloseTab),
        "reload" | "r" => Ok(BrowserAction::Reload),
        "back" => Ok(BrowserAction::GoBack),
        "forward" => Ok(BrowserAction::GoForward),
        "zoom" => parse_zoom_arg(rest),
        "inspect" | "inspector" | "devtools" => Ok(BrowserAction::ToggleInspector),
        "help" | "h" => Ok(BrowserAction::ShowHelp),
        "w" | "write" => parse_optional_session(rest).map(BrowserAction::SaveSession),
        "wq" => parse_optional_session(rest).map(BrowserAction::SaveSessionAndQuit),
        "session" => parse_required_session(rest).map(BrowserAction::LoadSession),
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

/// Normalize user-entered targets into navigable URLs.
///
/// Explicit schemes, `localhost`, IP literals, and bare domain names resolve
/// directly to URLs; anything else falls back to a Google Search query.
pub fn normalize_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if has_explicit_scheme(trimmed) {
        return trimmed.to_string();
    }

    if !trimmed.chars().any(char::is_whitespace)
        && let Some(scheme) = infer_scheme_for_bare_target(trimmed)
    {
        return format!("{scheme}{trimmed}");
    }

    google_search_url(trimmed)
}

fn has_explicit_scheme(input: &str) -> bool {
    const STRICT_SCHEMES: &[&str] = &["http://", "https://", "about:", "view-source:"];
    const FLEXIBLE_SCHEMES: &[&str] = &["file://", "data:"];

    if STRICT_SCHEMES.iter().any(|prefix| {
        starts_with_ascii_case(input, prefix)
            && input.len() > prefix.len()
            && !input.chars().any(char::is_whitespace)
    }) {
        return true;
    }

    FLEXIBLE_SCHEMES
        .iter()
        .any(|prefix| starts_with_ascii_case(input, prefix) && input.len() > prefix.len())
}

fn starts_with_ascii_case(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

fn infer_scheme_for_bare_target(input: &str) -> Option<&'static str> {
    let authority = input.split(['/', '?', '#']).next()?;
    if authority.is_empty() {
        return None;
    }

    if let Some(rest) = authority.strip_prefix('[') {
        let (addr, after_bracket) = rest.split_once(']')?;
        if !after_bracket.is_empty() && !is_valid_port_suffix(after_bracket) {
            return None;
        }
        return addr.parse::<Ipv6Addr>().ok().map(|_| "http://");
    }

    let (host, port_suffix) = match authority.find(':') {
        Some(idx) => (&authority[..idx], Some(&authority[idx..])),
        None => (authority, None),
    };
    if let Some(suffix) = port_suffix
        && !is_valid_port_suffix(suffix)
    {
        return None;
    }

    if host.eq_ignore_ascii_case("localhost") || host.parse::<Ipv4Addr>().is_ok() {
        return Some("http://");
    }
    if looks_like_domain(host) {
        return Some("https://");
    }
    None
}

fn is_valid_port_suffix(suffix: &str) -> bool {
    let Some(port) = suffix.strip_prefix(':') else {
        return false;
    };
    !port.is_empty()
        && port.bytes().all(|b| b.is_ascii_digit())
        && port.parse::<u16>().is_ok_and(|p| p > 0)
}

fn looks_like_domain(host: &str) -> bool {
    if !host.contains('.') {
        return false;
    }
    let labels: Vec<&str> = host.split('.').collect();
    let Some((&tld, prefix)) = labels.split_last() else {
        return false;
    };
    if prefix.is_empty() || !labels.iter().all(|label| is_valid_domain_label(label)) {
        return false;
    }
    is_valid_tld(tld)
}

fn is_valid_domain_label(label: &str) -> bool {
    (1..=63).contains(&label.len())
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

fn is_valid_tld(tld: &str) -> bool {
    (tld.len() >= 2 && tld.bytes().all(|b| b.is_ascii_alphabetic()))
        || (tld.len() > 4
            && starts_with_ascii_case(tld, "xn--")
            && tld[4..]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-'))
}

fn google_search_url(query: &str) -> String {
    let mut encoded = String::with_capacity(query.len() * 3);
    for &byte in query.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push('+'),
            other => {
                let _ = write!(&mut encoded, "%{other:02X}");
            }
        }
    }
    format!("https://www.google.com/search?q={encoded}")
}
