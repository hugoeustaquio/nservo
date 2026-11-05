use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub path: PathBuf,
    pub entry_file: PathBuf,
    pub trusted: bool,
}

impl PluginInfo {
    pub fn new(name: String, path: PathBuf, entry_file: PathBuf, trusted: bool) -> Self {
        Self {
            name,
            path,
            entry_file,
            trusted,
        }
    }
}
