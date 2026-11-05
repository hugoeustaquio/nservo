use crate::adblock::rule::{AdRule, ContentType, RuleType};

pub struct AdblockParser;

impl AdblockParser {
    pub fn parse_line(line: &str) -> Option<AdRule> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('!') || trimmed.starts_with('[') {
            return None;
        }

        // Element hiding rule: domain##selector or ##selector
        if let Some(pos) = trimmed.find("##") {
            let domains_part = &trimmed[..pos];
            let selector = &trimmed[pos + 2..];
            let mut rule = AdRule::new(RuleType::ElementHiding, selector.to_string());
            if !domains_part.is_empty() {
                rule.domains = domains_part.split(',').map(|s| s.trim().to_string()).collect();
            }
            return Some(rule);
        }

        // Exception rule: @@pattern$options
        let is_exception = trimmed.starts_with("@@");
        let rule_content = if is_exception { &trimmed[2..] } else { trimmed };

        let (pattern_part, options_part) = match rule_content.find('$') {
            Some(idx) => (&rule_content[..idx], Some(&rule_content[idx + 1..])),
            None => (rule_content, None),
        };

        let rule_type = if is_exception {
            RuleType::NetworkException
        } else {
            RuleType::NetworkBlock
        };

        let mut rule = AdRule::new(rule_type, pattern_part.to_string());

        if let Some(opts) = options_part {
            for opt in opts.split(',') {
                let opt_trim = opt.trim();
                if opt_trim == "third-party" {
                    rule.third_party = Some(true);
                } else if opt_trim == "~third-party" {
                    rule.third_party = Some(false);
                } else if opt_trim.starts_with("domain=") {
                    let domain_list = &opt_trim["domain=".len()..];
                    for d in domain_list.split('|') {
                        if d.starts_with('~') {
                            rule.excluded_domains.push(d[1..].to_string());
                        } else {
                            rule.domains.push(d.to_string());
                        }
                    }
                } else {
                    rule.content_types.push(ContentType::from_str(opt_trim));
                }
            }
        }

        Some(rule)
    }

    pub fn parse_filter_list(content: &str) -> Vec<AdRule> {
        content
            .lines()
            .filter_map(Self::parse_line)
            .collect()
    }
}
