use std::collections::HashSet;

pub(crate) use crate::core::adblock_parse::{
    ParsedOptions, extract_pure_domain, is_safe_css_selector, parse_hosts_line, parse_options,
    split_options,
};
pub use crate::core::adblock_parse::{
    TYPE_DOCUMENT, TYPE_FONT, TYPE_IMAGE, TYPE_MEDIA, TYPE_OTHER, TYPE_SCRIPT, TYPE_STYLESHEET,
    TYPE_SUBDOCUMENT, TYPE_XHR, destination_to_type_mask,
};

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
    if core.is_empty() {
        return None;
    }
    let pattern = if opts.match_case {
        core.to_string()
    } else {
        core.to_ascii_lowercase()
    };
    let literal_hint = pattern
        .split(['*', '^'])
        .max_by_key(|s| s.len())
        .unwrap_or("")
        .to_string();
    Some(NetworkRule {
        pattern,
        anchor_domain,
        anchor_start,
        anchor_end,
        match_case: opts.match_case,
        literal_hint,
        third_party: opts.third_party,
        type_include: opts.type_include,
        type_exclude: opts.type_exclude,
        include_domains: opts.include_domains,
        exclude_domains: opts.exclude_domains,
    })
}

impl NetworkRule {
    pub fn matches(
        &self,
        url: &str,
        url_lower: &str,
        src_host: &str,
        is_tp: bool,
        ty: u16,
    ) -> bool {
        if let Some(req_tp) = self.third_party
            && req_tp != is_tp
        {
            return false;
        }
        if self.type_include != 0 && (ty & self.type_include) == 0 {
            return false;
        }
        if self.type_exclude != 0 && (ty & self.type_exclude) != 0 {
            return false;
        }
        if !self.include_domains.is_empty()
            && !self
                .include_domains
                .iter()
                .any(|d| host_matches_domain(src_host, d))
        {
            return false;
        }
        if self
            .exclude_domains
            .iter()
            .any(|d| host_matches_domain(src_host, d))
        {
            return false;
        }
        let hay = if self.match_case { url } else { url_lower };
        if !self.literal_hint.is_empty() && !hay.contains(&self.literal_hint) {
            return false;
        }
        if self.anchor_domain {
            let Some(after_scheme) = hay.find("://").map(|i| &hay[i + 3..]) else {
                return false;
            };
            let host_end = after_scheme
                .find(['/', '?', '#'])
                .unwrap_or(after_scheme.len());
            let mut offset = 0usize;
            while offset < host_end {
                if abp_match(
                    &self.pattern,
                    &after_scheme[offset..],
                    true,
                    self.anchor_end,
                ) {
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
    (0..=text.len())
        .any(|i| text.is_char_boundary(i) && match_parts(&parts, &text[i..], anchor_end))
}

fn match_parts(parts: &[&str], text: &str, anchor_end: bool) -> bool {
    let Some((&first, rest)) = parts.split_first() else {
        return !anchor_end || text.is_empty();
    };
    let Some(after) = match_segment_at_start(first, text) else {
        return false;
    };
    if rest.is_empty() {
        return !anchor_end || after.is_empty();
    }
    (0..=after.len())
        .any(|i| after.is_char_boundary(i) && match_parts(rest, &after[i..], anchor_end))
}

fn match_segment_at_start<'a>(seg: &str, text: &'a str) -> Option<&'a str> {
    let mut ti = 0usize;
    let tb = text.as_bytes();
    for &p in seg.as_bytes() {
        if p == b'^' {
            if ti == tb.len() {
                continue;
            }
            if is_separator(tb[ti]) {
                ti += 1;
                continue;
            }
            return None;
        }
        if ti >= tb.len() || tb[ti] != p {
            return None;
        }
        ti += 1;
    }
    Some(&text[ti..])
}

fn is_separator(b: u8) -> bool {
    !(b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'%'))
}

pub(crate) fn is_exempt_scheme(url: &str) -> bool {
    let u = url.trim();
    u.starts_with("about:")
        || u.starts_with("data:")
        || u.starts_with("file:")
        || u.starts_with("blob:")
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
    if labels.len() >= 2 {
        format!("{}.{}", labels[labels.len() - 2], labels[labels.len() - 1])
    } else {
        lower
    }
}

fn host_matches_domain(host: &str, domain: &str) -> bool {
    let h = host.to_ascii_lowercase();
    h == domain
        || (h.len() > domain.len()
            && h.ends_with(domain)
            && h.as_bytes()[h.len() - domain.len() - 1] == b'.')
}

pub(crate) fn matches_domain_set(set: &HashSet<String>, host: &str) -> bool {
    if set.is_empty() || host.is_empty() {
        return false;
    }
    let lower = host.to_ascii_lowercase();
    let mut slice = lower.as_str();
    loop {
        if set.contains(slice) {
            return true;
        }
        match slice.find('.') {
            Some(dot) => slice = &slice[dot + 1..],
            None => return false,
        }
    }
}
