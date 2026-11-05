use std::path::Path;
use crate::error::Result;
use crate::greasemonkey::metadata::ScriptMetadata;
use crate::greasemonkey::script::UserScript;

pub struct GreasemonkeyManager {
    scripts: Vec<UserScript>,
}

impl GreasemonkeyManager {
    pub fn new() -> Self {
        Self { scripts: Vec::new() }
    }

    pub fn load_directory(&mut self, dir: &Path) -> Result<usize> {
        if !dir.exists() {
            return Ok(0);
        }

        let mut loaded = 0;
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.to_string_lossy().ends_with(".user.js") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Some(metadata) = ScriptMetadata::parse(&content) {
                        self.scripts.push(UserScript::new(path, metadata, content));
                        loaded += 1;
                    }
                }
            }
        }
        Ok(loaded)
    }

    pub fn get_scripts_for_url(&self, url: &str) -> Vec<&UserScript> {
        self.scripts
            .iter()
            .filter(|script| script.matches_url(url))
            .collect()
    }

    pub fn scripts_count(&self) -> usize {
        self.scripts.len()
    }
}

impl Default for GreasemonkeyManager {
    fn default() -> Self {
        Self::new()
    }
}
