use std::path::Path;
use crate::adblock::parser::AdblockParser;
use crate::adblock::rule::{AdRule, ContentType, RuleType};
use crate::error::Result;

pub struct AdblockEngine {
    rules: Vec<AdRule>,
}

impl AdblockEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn load_from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let content = std::fs::read_to_string(path)?;
        let rules = AdblockParser::parse_filter_list(&content);
        Ok(Self { rules })
    }

    pub fn load_rules_from_str(&mut self, content: &str) {
        let mut new_rules = AdblockParser::parse_filter_list(content);
        self.rules.append(&mut new_rules);
    }

    pub fn rules_count(&self) -> usize {
        self.rules.len()
    }

    /// Check if a network request URL should be blocked.
    pub fn should_block_request(&self, url: &str, content_type: ContentType, is_third_party: bool) -> bool {
        // First check exception rules (whitelist)
        for rule in &self.rules {
            if rule.rule_type == RuleType::NetworkException && self.matches_network(rule, url, content_type, is_third_party) {
                return false;
            }
        }

        // Then check blocking rules
        for rule in &self.rules {
            if rule.rule_type == RuleType::NetworkBlock && self.matches_network(rule, url, content_type, is_third_party) {
                return true;
            }
        }

        false
    }

    fn matches_network(&self, rule: &AdRule, url: &str, content_type: ContentType, is_third_party: bool) -> bool {
        if let Some(tp) = rule.third_party {
            if tp != is_third_party {
                return false;
            }
        }

        if !rule.content_types.is_empty() && !rule.content_types.contains(&content_type) {
            return false;
        }

        let pattern = &rule.pattern;
        if pattern.starts_with("||") {
            let domain_part = &pattern[2..].trim_end_matches('^');
            url.contains(domain_part)
        } else if pattern.starts_with('|') {
            url.starts_with(&pattern[1..])
        } else if pattern.ends_with('|') {
            url.ends_with(&pattern[..pattern.len() - 1])
        } else {
            url.contains(pattern)
        }
    }

    /// Retrieve CSS selectors to hide for a specific domain.
    pub fn get_element_hiding_selectors(&self, domain: &str) -> Vec<String> {
        let mut selectors = Vec::new();
        for rule in &self.rules {
            if rule.rule_type == RuleType::ElementHiding {
                if rule.domains.is_empty() || rule.domains.iter().any(|d| domain.ends_with(d)) {
                    selectors.push(rule.pattern.clone());
                }
            }
        }
        selectors
    }
}

impl Default for AdblockEngine {
    fn default() -> Self {
        Self::new()
    }
}
