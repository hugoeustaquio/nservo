use std::collections::HashMap;
use std::path::PathBuf;
use crate::error::{NServoError, Result};
use crate::plugins::plugin::PluginInfo;
use crate::security::PluginTrustStore;

pub struct PluginManager {
    plugins_dir: PathBuf,
    discovered: HashMap<String, PluginInfo>,
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins_dir,
            discovered: HashMap::new(),
        }
    }

    pub fn discover(&mut self, trust_store: &PluginTrustStore) -> Result<usize> {
        if !self.plugins_dir.exists() {
            return Ok(0);
        }

        self.discovered.clear();
        for entry in std::fs::read_dir(&self.plugins_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let init_file = path.join("init.lua");
                if init_file.exists() {
                    let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let content = std::fs::read_to_string(&init_file).unwrap_or_default();
                    let trusted = trust_store.is_trusted(&name, &content);

                    self.discovered.insert(
                        name.clone(),
                        PluginInfo::new(name, path, init_file, trusted),
                    );
                }
            }
        }

        Ok(self.discovered.len())
    }

    pub fn get(&self, name: &str) -> Option<&PluginInfo> {
        self.discovered.get(name)
    }

    pub fn list(&self) -> Vec<&PluginInfo> {
        self.discovered.values().collect()
    }

    pub fn load_plugin_script(&self, name: &str, trust_store: &PluginTrustStore) -> Result<String> {
        let plugin = self.discovered.get(name).ok_or_else(|| NServoError::Plugin {
            name: name.to_string(),
            reason: "Plugin not found in plugins directory".to_string(),
        })?;

        let content = std::fs::read_to_string(&plugin.entry_file)?;
        if !trust_store.is_trusted(name, &content) {
            return Err(NServoError::Security(format!(
                "Plugin '{}' is not trusted. Use the security prompt to approve it.",
                name
            )));
        }

        Ok(content)
    }
}
