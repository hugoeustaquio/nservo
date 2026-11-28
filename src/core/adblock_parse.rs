/// Bitmask constants for ABP resource type options.
pub const TYPE_SCRIPT: u16 = 1 << 0;
pub const TYPE_IMAGE: u16 = 1 << 1;
pub const TYPE_STYLESHEET: u16 = 1 << 2;
pub const TYPE_XHR: u16 = 1 << 3;
pub const TYPE_SUBDOCUMENT: u16 = 1 << 4;
pub const TYPE_MEDIA: u16 = 1 << 5;
pub const TYPE_FONT: u16 = 1 << 6;
pub const TYPE_DOCUMENT: u16 = 1 << 7;
pub const TYPE_OTHER: u16 = 1 << 8;

#[derive(Default)]
pub(crate) struct ParsedOptions {
    pub third_party: Option<bool>,
    pub match_case: bool,
    pub type_include: u16,
    pub type_exclude: u16,
    pub include_domains: Vec<String>,
    pub exclude_domains: Vec<String>,
}

impl ParsedOptions {
    pub fn is_domain_fast_path(&self) -> bool {
        !self.match_case
            && self.type_include == 0
            && self.type_exclude == 0
            && self.include_domains.is_empty()
            && self.exclude_domains.is_empty()
    }
}

/// Map a Fetch `Destination` debug/name string to an ABP resource type bitmask.
pub fn destination_to_type_mask(dest: &str) -> u16 {
    match dest.to_ascii_lowercase().as_str() {
        "script" | "worker" | "sharedworker" | "serviceworker" => TYPE_SCRIPT,
        "image" => TYPE_IMAGE,
        "style" | "stylesheet" => TYPE_STYLESHEET,
        "iframe" | "frame" | "subdocument" => TYPE_SUBDOCUMENT,
        "audio" | "video" | "track" | "media" => TYPE_MEDIA,
        "font" => TYPE_FONT,
        "document" => TYPE_DOCUMENT,
        "" | "none" | "empty" | "xmlhttprequest" | "xhr" => TYPE_XHR | TYPE_OTHER,
        _ => TYPE_OTHER,
    }
}

pub(crate) fn split_options(body: &str) -> (&str, Option<&str>) {
    let Some(idx) = body.rfind('$') else {
        return (body, None);
    };
    let candidate = &body[idx + 1..];
    if candidate.is_empty() || candidate.contains('/') {
        return (body, None);
    }
    (&body[..idx], Some(candidate))
}

pub(crate) fn parse_options(raw: Option<&str>) -> Option<ParsedOptions> {
    let mut opts = ParsedOptions::default();
    let Some(raw) = raw else {
        return Some(opts);
    };
    for token in raw.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        if let Some(doms) = token.strip_prefix("domain=") {
            for d in doms.split('|').filter(|d| !d.is_empty()) {
                if let Some(exc) = d.strip_prefix('~') {
                    opts.exclude_domains.push(exc.to_ascii_lowercase());
                } else {
                    opts.include_domains.push(d.to_ascii_lowercase());
                }
            }
            continue;
        }
        let (neg, name) = match token.strip_prefix('~') {
            Some(rest) => (true, rest),
            None => (false, token),
        };
        match name {
            "third-party" | "3p" => opts.third_party = Some(!neg),
            "first-party" | "1p" => opts.third_party = Some(neg),
            "match-case" => opts.match_case = !neg,
            "important" | "all" => {}
            other => {
                if let Some(bit) = option_type_bit(other) {
                    if neg {
                        opts.type_exclude |= bit;
                    } else {
                        opts.type_include |= bit;
                    }
                } else if !neg {
                    // Do not drop unsupported positive constraints (e.g. $popup, $generichide,
                    // $object, $csp=..., $rewrite=...), which would over-match unrelated loads.
                    return None;
                }
            }
        }
    }
    Some(opts)
}

fn option_type_bit(name: &str) -> Option<u16> {
    Some(match name {
        "script" => TYPE_SCRIPT,
        "image" => TYPE_IMAGE,
        "stylesheet" | "css" => TYPE_STYLESHEET,
        "xmlhttprequest" | "xhr" | "fetch" => TYPE_XHR,
        "subdocument" | "frame" => TYPE_SUBDOCUMENT,
        "media" => TYPE_MEDIA,
        "font" => TYPE_FONT,
        "document" => TYPE_DOCUMENT,
        "other" | "ping" | "websocket" => TYPE_OTHER,
        _ => return None,
    })
}

pub(crate) fn parse_hosts_line(line: &str) -> Option<String> {
    let mut parts = line.split_whitespace();
    let ip = parts.next()?;
    if ip != "0.0.0.0" && ip != "127.0.0.1" {
        return None;
    }
    let host = parts.next()?.split('#').next()?.trim().to_ascii_lowercase();
    (!host.is_empty() && host != "localhost" && host.contains('.')).then_some(host)
}

pub(crate) fn is_safe_css_selector(sel: &str) -> bool {
    let s = sel.trim();
    !s.is_empty()
        && !s.contains(['{', '}'])
        && !s.contains(":-abp-")
        && !s.contains(":has-text(")
        && !s.contains(":xpath(")
        && !s.contains("+js(")
        && !s.contains(":style(")
}

pub(crate) fn extract_pure_domain(pattern: &str) -> Option<String> {
    let domain = pattern.strip_prefix("||")?.strip_suffix('^')?;
    let valid = !domain.is_empty()
        && domain.contains('.')
        && domain
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-');
    valid.then(|| domain.to_ascii_lowercase())
}
