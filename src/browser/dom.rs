/// Lightweight DOM node representation and HTML parser.
#[derive(Debug, Clone, PartialEq)]
pub enum DomNodeType {
    Document,
    Element { tag: String, href: Option<String> },
    Text(String),
}

#[derive(Debug, Clone)]
pub struct DomNode {
    pub node_type: DomNodeType,
    pub children: Vec<DomNode>,
}

impl DomNode {
    pub fn new(node_type: DomNodeType) -> Self {
        Self {
            node_type,
            children: Vec::new(),
        }
    }

    pub fn text(content: &str) -> Self {
        Self::new(DomNodeType::Text(content.to_string()))
    }

    pub fn element(tag: &str, href: Option<String>) -> Self {
        Self::new(DomNodeType::Element {
            tag: tag.to_lowercase(),
            href,
        })
    }

    pub fn parse_html(html: &str) -> Self {
        let mut root = DomNode::new(DomNodeType::Document);
        let mut chars = html.chars().peekable();
        let mut current_text = String::new();

        while let Some(&c) = chars.peek() {
            if c == '<' {
                if !current_text.trim().is_empty() {
                    root.children.push(DomNode::text(current_text.trim()));
                    current_text.clear();
                }
                chars.next(); // consume '<'

                let mut tag_content = String::new();
                while let Some(&tc) = chars.peek() {
                    chars.next();
                    if tc == '>' {
                        break;
                    }
                    tag_content.push(tc);
                }

                let tag_trimmed = tag_content.trim();
                if !tag_trimmed.starts_with('/') && !tag_trimmed.starts_with('!') {
                    let mut parts = tag_trimmed.split_whitespace();
                    let tag_name = parts.next().unwrap_or("");
                    let mut href = None;
                    for part in parts {
                        if part.starts_with("href=") {
                            let clean_href = part["href=".len()..]
                                .trim_matches('"')
                                .trim_matches('\'');
                            href = Some(clean_href.to_string());
                        }
                    }

                    // Ignore script and style blocks
                    if tag_name.eq_ignore_ascii_case("script") || tag_name.eq_ignore_ascii_case("style") {
                        Self::skip_until_closing(&mut chars, tag_name);
                    } else if !tag_name.is_empty() {
                        root.children.push(DomNode::element(tag_name, href));
                    }
                }
            } else {
                chars.next();
                current_text.push(c);
            }
        }

        if !current_text.trim().is_empty() {
            root.children.push(DomNode::text(current_text.trim()));
        }

        root
    }

    fn skip_until_closing<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>, tag: &str) {
        let closing = format!("</{}", tag);
        let mut buffer = String::new();
        while let Some(c) = chars.next() {
            buffer.push(c);
            if buffer.to_lowercase().ends_with(&closing) {
                while let Some(&nc) = chars.peek() {
                    chars.next();
                    if nc == '>' {
                        break;
                    }
                }
                break;
            }
        }
    }

    pub fn extract_title(&self) -> Option<String> {
        let mut title_found = false;
        for child in &self.children {
            if let DomNodeType::Element { ref tag, .. } = child.node_type {
                if tag == "title" {
                    title_found = true;
                    continue;
                }
            }
            if title_found {
                if let DomNodeType::Text(ref text) = child.node_type {
                    return Some(text.clone());
                }
            }
        }
        None
    }
}
