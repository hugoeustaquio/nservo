use std::collections::HashSet;

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

#[derive(Debug, Clone)]
pub(crate) struct NetworkRule {
    pattern: String,
    anchor_domain: bool,
    anchor_start: bool,
    anchor_end: bool,
    match_case: bool,
    literal_hint: String,
    third_party: Option<bool>,
    type_include: u16,
    type_exclude: u16,
    include_domains: Vec<String>,
    exclude_domains: Vec<String>,
}

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
    let Some(idx) = body.rfind('$') else { return (body, None); };
    let candidate = &body[idx + 1..];
    if candidate.is_empty() || candidate.contains('/') {
        return (body, None);
    }
    (&body[..idx], Some(candidate))
}

pub(crate) fn parse_options(raw: Option<&str>) -> Option<ParsedOptions> {
    let mut opts = ParsedOptions::default();
    let Some(raw) = raw else { return Some(opts); };
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
                    if neg { opts.type_exclude |= bit; } else { opts.type_include |= bit; }
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
    if ip != "0.0.0.0" && ip != "127.0.0.1" { return None; }
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
        && domain.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-');
    valid.then(|| domain.to_ascii_lowercase())
}

pub(crate) fn build_network_rule(raw: &str, opts: ParsedOptions) -> Option<NetworkRule> {
    let (anchor_domain, anchor_start, rest) = if let Some(r) = raw.strip_prefix("||") {
        (true, false, r)
    } else if let Some(r) = raw.strip_prefix('|') {
        (false, true, r)
    } else {
        (false, false, raw)
    };
    let (anchor_end, core) = match rest.strip_suffix('|') {
        Some(r) => (true, r),
        None => (false, rest),
    };
    if core.is_empty() { return None; }
    let pattern = if opts.match_case { core.to_string() } else { core.to_ascii_lowercase() };
    let literal_hint = pattern.split(['*', '^']).max_by_key(|s| s.len()).unwrap_or("").to_string();
    Some(NetworkRule {
        pattern, anchor_domain, anchor_start, anchor_end, match_case: opts.match_case,
        literal_hint, third_party: opts.third_party, type_include: opts.type_include,
        type_exclude: opts.type_exclude, include_domains: opts.include_domains,
        exclude_domains: opts.exclude_domains,
    })
}

impl NetworkRule {
    pub fn matches(&self, url: &str, url_lower: &str, src_host: &str, is_tp: bool, ty: u16) -> bool {
        if let Some(req_tp) = self.third_party && req_tp != is_tp { return false; }
        if self.type_include != 0 && (ty & self.type_include) == 0 { return false; }
        if self.type_exclude != 0 && (ty & self.type_exclude) != 0 { return false; }
        if !self.include_domains.is_empty()
            && !self.include_domains.iter().any(|d| host_matches_domain(src_host, d))
        {
            return false;
        }
        if self.exclude_domains.iter().any(|d| host_matches_domain(src_host, d)) { return false; }
        let hay = if self.match_case { url } else { url_lower };
        if !self.literal_hint.is_empty() && !hay.contains(&self.literal_hint) { return false; }
        if self.anchor_domain {
            let Some(after_scheme) = hay.find("://").map(|i| &hay[i + 3..]) else { return false; };
            let host_end = after_scheme.find(['/', '?', '#']).unwrap_or(after_scheme.len());
            let mut offset = 0usize;
            while offset < host_end {
                if abp_match(&self.pattern, &after_scheme[offset..], true, self.anchor_end) {
                    return true;
                }
                match after_scheme[offset..host_end].find('.') {
                    Some(dot) => offset += dot + 1,
                    None => break,
                }
            }
            false
        } else {
            abp_match(&self.pattern, hay, self.anchor_start, self.anchor_end)
        }
    }
}

fn abp_match(pat: &str, text: &str, anchor_start: bool, anchor_end: bool) -> bool {
    let parts: Vec<&str> = pat.split('*').collect();
    if anchor_start {
        return match_parts(&parts, text, anchor_end);
    }
    if parts[0].is_empty() {
        return match_parts(&parts[1..], text, anchor_end);
    }
    (0..=text.len()).any(|i| text.is_char_boundary(i) && match_parts(&parts, &text[i..], anchor_end))
}

fn match_parts(parts: &[&str], text: &str, anchor_end: bool) -> bool {
    let Some((&first, rest)) = parts.split_first() else { return !anchor_end || text.is_empty(); };
    let Some(after) = match_segment_at_start(first, text) else { return false; };
    if rest.is_empty() { return !anchor_end || after.is_empty(); }
    (0..=after.len()).any(|i| after.is_char_boundary(i) && match_parts(rest, &after[i..], anchor_end))
}

fn match_segment_at_start<'a>(seg: &str, text: &'a str) -> Option<&'a str> {
    let mut ti = 0usize;
    let tb = text.as_bytes();
    for &p in seg.as_bytes() {
        if p == b'^' {
            if ti == tb.len() { continue; }
            if is_separator(tb[ti]) { ti += 1; continue; }
            return None;
        }
        if ti >= tb.len() || tb[ti] != p { return None; }
        ti += 1;
    }
    Some(&text[ti..])
}

fn is_separator(b: u8) -> bool {
    !(b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'%'))
}

pub(crate) fn is_exempt_scheme(url: &str) -> bool {
    let u = url.trim();
    u.starts_with("about:") || u.starts_with("data:") || u.starts_with("file:") || u.starts_with("blob:")
}

pub(crate) fn extract_host(url: &str) -> Option<&str> {
    let after = url.split_once("://")?.1;
    let authority = after.split(['/', '?', '#']).next()?;
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = host_port.split(':').next()?;
    (!host.is_empty()).then_some(host)
}

pub(crate) fn base_domain(host: &str) -> String {
    let lower = host.to_ascii_lowercase();
    let labels: Vec<&str> = lower.split('.').collect();
    if labels.len() >= 2 { format!("{}.{}", labels[labels.len() - 2], labels[labels.len() - 1]) } else { lower }
}

fn host_matches_domain(host: &str, domain: &str) -> bool {
    let h = host.to_ascii_lowercase();
    h == domain
        || (h.len() > domain.len()
            && h.ends_with(domain)
            && h.as_bytes()[h.len() - domain.len() - 1] == b'.')
}

pub(crate) fn matches_domain_set(set: &HashSet<String>, host: &str) -> bool {
    if set.is_empty() || host.is_empty() { return false; }
    let lower = host.to_ascii_lowercase();
    let mut slice = lower.as_str();
    loop {
        if set.contains(slice) { return true; }
        match slice.find('.') {
            Some(dot) => slice = &slice[dot + 1..],
            None => return false,
        }
    }
}
