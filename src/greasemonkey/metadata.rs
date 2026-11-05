#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunAt {
    DocumentStart,
    DocumentEnd,
    DocumentIdle,
}

impl Default for RunAt {
    fn default() -> Self {
        RunAt::DocumentEnd
    }
}

/// Metadata extracted from a `// ==UserScript==` block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptMetadata {
    pub name: String,
    pub description: String,
    pub matches: Vec<String>,
    pub includes: Vec<String>,
    pub run_at: RunAt,
    pub grant: Vec<String>,
}

impl ScriptMetadata {
    pub fn parse(content: &str) -> Option<Self> {
        let start_tag = "// ==UserScript==";
        let end_tag = "// ==/UserScript==";

        let start_pos = content.find(start_tag)?;
        let end_pos = content.find(end_tag)?;
        if end_pos <= start_pos {
            return None;
        }

        let block = &content[start_pos + start_tag.len()..end_pos];
        let mut meta = ScriptMetadata::default();

        for line in block.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("//") {
                continue;
            }
            let text = trimmed.trim_start_matches('/').trim();
            if !text.starts_with('@') {
                continue;
            }

            let mut parts = text[1..].split_whitespace();
            let key = parts.next().unwrap_or("");
            let val = parts.collect::<Vec<&str>>().join(" ");

            match key {
                "name" => meta.name = val,
                "description" => meta.description = val,
                "match" => meta.matches.push(val),
                "include" => meta.includes.push(val),
                "grant" => meta.grant.push(val),
                "run-at" => {
                    meta.run_at = match val.as_str() {
                        "document-start" => RunAt::DocumentStart,
                        "document-idle" => RunAt::DocumentIdle,
                        _ => RunAt::DocumentEnd,
                    };
                }
                _ => {}
            }
        }

        Some(meta)
    }
}
