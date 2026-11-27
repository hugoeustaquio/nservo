use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::adblock_match::{
    NetworkRule, base_domain, build_network_rule, extract_host, extract_pure_domain,
    is_exempt_scheme, is_safe_css_selector, matches_domain_set, parse_hosts_line, parse_options,
    split_options,
};

pub use crate::core::adblock_match::{
    TYPE_DOCUMENT, TYPE_FONT, TYPE_IMAGE, TYPE_MEDIA, TYPE_OTHER, TYPE_SCRIPT, TYPE_STYLESHEET,
    TYPE_SUBDOCUMENT, TYPE_XHR, destination_to_type_mask,
};

/// Pure ABP-like filter list parser, matcher, and `$XDG_CONFIG_HOME/nservo/adblock/` helpers.
#[derive(Debug, Default, Clone)]
pub struct AdblockFilter {
    blocked_domains: HashSet<String>,
    blocked_tp_domains: HashSet<String>,
    allowed_domains: HashSet<String>,
    block_rules: Vec<NetworkRule>,
    allow_rules: Vec<NetworkRule>,
    cosmetic_selectors: Vec<String>,
    rule_count: usize,
}

/// Encode a filter-list `url` into a single reversible filename for `adblock/<URL>`.
pub fn encode_filter_filename(url: &str) -> String {
    let mut out = String::with_capacity(url.len() + 16);
    for &b in url.as_bytes() {
        match b {
            b'/' | b'\\' | b'%' | 0 => {
                let _ = write!(&mut out, "%{b:02X}");
            }
            _ => out.push(b as char),
        }
    }
    out
}

/// Decode a filename produced by [`encode_filter_filename`] back into its original URL.
pub fn decode_filter_filename(name: &str) -> Option<String> {
    let bytes = name.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16)?;
            let lo = (bytes[i + 2] as char).to_digit(16)?;
            out.push(((hi << 4) | lo) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    let decoded = String::from_utf8(out).ok()?;
    (decoded.starts_with("http://") || decoded.starts_with("https://")).then_some(decoded)
}

/// Full path where `url` is stored under `adblock_dir`.
pub fn filter_file_path(adblock_dir: &Path, url: &str) -> PathBuf {
    adblock_dir.join(encode_filter_filename(url))
}

/// Discover all saved filter files in `adblock_dir` along with their decoded source URL (if any).
pub fn list_filter_files(adblock_dir: &Path) -> Vec<(PathBuf, Option<String>)> {
    let Ok(entries) = fs::read_dir(adblock_dir) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') || !path.is_file() {
            continue;
        }
        let url = decode_filter_filename(&name_str);
        files.push((path, url));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

impl AdblockFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load and parse every filter list file inside `adblock_dir`.
    pub fn load_from_dir(adblock_dir: &Path) -> (Self, usize) {
        let mut engine = Self::new();
        let files = list_filter_files(adblock_dir);
        let mut loaded = 0usize;
        for (path, _) in files {
            if let Ok(text) = fs::read_to_string(&path) {
                engine.add_list(&text);
                loaded += 1;
            }
        }
        (engine, loaded)
    }

    pub fn rule_count(&self) -> usize {
        self.rule_count
    }

    pub fn cosmetic_selectors(&self) -> &[String] {
        &self.cosmetic_selectors
    }

    /// Parse all lines of an ABP-like (or hosts-style) filter list into this engine.
    pub fn add_list(&mut self, text: &str) {
        for raw in text.lines() {
            self.add_line(raw);
        }
    }

    /// Parse a single filter line.
    pub fn add_line(&mut self, raw: &str) {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
            return;
        }
        if let Some(domain) = parse_hosts_line(line) {
            self.blocked_domains.insert(domain);
            self.rule_count += 1;
            return;
        }
        if line.starts_with('#') {
            if let Some(sel) = line.strip_prefix("##")
                && is_safe_css_selector(sel)
            {
                self.cosmetic_selectors.push(sel.trim().to_string());
                self.rule_count += 1;
            }
            return;
        }
        if line.contains("##")
            || line.contains("#@#")
            || line.contains("#?#")
            || line.contains("#%#")
            || line.contains("#$#")
            || line.contains("#@$#")
        {
            return;
        }

        let (is_allow, body) = match line.strip_prefix("@@") {
            Some(rest) => (true, rest),
            None => (false, line),
        };
        if body.is_empty() {
            return;
        }
        let (pattern_raw, options_raw) = split_options(body);
        if pattern_raw.is_empty()
            || (pattern_raw.starts_with('/') && pattern_raw.ends_with('/') && pattern_raw.len() > 2)
        {
            return;
        }
        let Some(opts) = parse_options(options_raw) else {
            return;
        };

        // Fast-path pure `||domain^` rules into O(labels) HashSets.
        if let Some(domain) = extract_pure_domain(pattern_raw)
            && opts.is_domain_fast_path()
        {
            if is_allow && opts.third_party.is_none() {
                self.allowed_domains.insert(domain);
                self.rule_count += 1;
                return;
            }
            if !is_allow && opts.third_party == Some(true) {
                self.blocked_tp_domains.insert(domain);
                self.rule_count += 1;
                return;
            }
            if !is_allow && opts.third_party.is_none() {
                self.blocked_domains.insert(domain);
                self.rule_count += 1;
                return;
            }
        }

        let Some(rule) = build_network_rule(pattern_raw, opts) else {
            return;
        };
        if is_allow {
            self.allow_rules.push(rule);
        } else {
            self.block_rules.push(rule);
        }
        self.rule_count += 1;
    }

    /// Return `true` if `url` should be blocked given optional `referrer_url` and resource `type_mask`.
    pub fn should_block(&self, url: &str, referrer_url: Option<&str>, type_mask: u16) -> bool {
        if self.rule_count == 0 || is_exempt_scheme(url) {
            return false;
        }
        let host = extract_host(url).unwrap_or("");
        let source_host = referrer_url.and_then(extract_host).unwrap_or("");
        let is_third_party = !source_host.is_empty()
            && !host.is_empty()
            && base_domain(host) != base_domain(source_host);
        let url_lower = url.to_ascii_lowercase();

        if matches_domain_set(&self.allowed_domains, host)
            || self
                .allow_rules
                .iter()
                .any(|r| r.matches(url, &url_lower, source_host, is_third_party, type_mask))
        {
            return false;
        }
        if matches_domain_set(&self.blocked_domains, host)
            || (is_third_party && matches_domain_set(&self.blocked_tp_domains, host))
        {
            return true;
        }
        self.block_rules
            .iter()
            .any(|r| r.matches(url, &url_lower, source_host, is_third_party, type_mask))
    }

    /// Build a capped CSS stylesheet hiding global `##selector` cosmetic rules.
    pub fn cosmetic_css(&self, max_rules: usize) -> String {
        let mut css = String::new();
        for sel in self.cosmetic_selectors.iter().take(max_rules) {
            let _ = writeln!(&mut css, "{sel} {{ display: none !important; }}");
        }
        css
    }
}
