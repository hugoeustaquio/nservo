/// Content type classification for network requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Script,
    Image,
    Subdocument, // iframe
    Stylesheet,
    XmlHttpRequest,
    Other,
}

impl ContentType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "script" => ContentType::Script,
            "image" => ContentType::Image,
            "subdocument" | "iframe" => ContentType::Subdocument,
            "stylesheet" | "css" => ContentType::Stylesheet,
            "xmlhttprequest" | "xhr" => ContentType::XmlHttpRequest,
            _ => ContentType::Other,
        }
    }
}

/// Type of rule: block network, whitelist exception, or element hiding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleType {
    NetworkBlock,
    NetworkException,
    ElementHiding,
}

/// Structured representation of an ABP-like rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdRule {
    pub rule_type: RuleType,
    pub pattern: String,
    pub domains: Vec<String>,
    pub excluded_domains: Vec<String>,
    pub content_types: Vec<ContentType>,
    pub third_party: Option<bool>,
}

impl AdRule {
    pub fn new(rule_type: RuleType, pattern: String) -> Self {
        Self {
            rule_type,
            pattern,
            domains: Vec::new(),
            excluded_domains: Vec::new(),
            content_types: Vec::new(),
            third_party: None,
        }
    }
}
