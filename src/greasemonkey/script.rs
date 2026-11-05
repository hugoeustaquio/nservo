use std::path::PathBuf;
use crate::greasemonkey::metadata::ScriptMetadata;

/// Representation of a parsed Greasemonkey user script.
#[derive(Debug, Clone)]
pub struct UserScript {
    pub file_path: PathBuf,
    pub metadata: ScriptMetadata,
    pub code: String,
}

impl UserScript {
    pub fn new(path: PathBuf, metadata: ScriptMetadata, code: String) -> Self {
        Self {
            file_path: path,
            metadata,
            code,
        }
    }

    /// Checks if this script should execute on the target URL.
    pub fn matches_url(&self, url: &str) -> bool {
        if self.metadata.matches.is_empty() && self.metadata.includes.is_empty() {
            return false;
        }

        for pattern in self.metadata.matches.iter().chain(self.metadata.includes.iter()) {
            if pattern == "*://*/*" || pattern == "*" {
                return true;
            }

            let mut pat = pattern.as_str();
            for prefix in &["*://", "http://", "https://"] {
                if let Some(rest) = pat.strip_prefix(prefix) {
                    pat = rest;
                    break;
                }
            }

            let clean_pat = pat.trim_end_matches("/*").trim_end_matches('*');
            if !clean_pat.is_empty() && url.contains(clean_pat) {
                return true;
            }
        }

        false
    }

    /// Builds a wrapped JavaScript execution payload.
    pub fn generate_injection_code(&self) -> String {
        format!(
            "(() => {{\n// NServo Greasemonkey Script: {}\ntry {{\n{}\n}} catch(e) {{\n  console.error('[NServo Greasemonkey] Error in script {}:', e);\n}}\n}})();",
            self.metadata.name, self.code, self.metadata.name
        )
    }
}
