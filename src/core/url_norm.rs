use std::fmt::Write;
use std::net::{Ipv4Addr, Ipv6Addr};

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

pub(crate) fn starts_with_ascii_case(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
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
