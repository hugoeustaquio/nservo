use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedPluginRecord {
    pub name: String,
    pub path: String,
    pub trusted_at: i64,
    pub content_hash: String,
}

pub struct PluginTrustStore {
    path: PathBuf,
    trusted: HashMap<String, TrustedPluginRecord>,
}

impl PluginTrustStore {
    pub fn load_or_create(path: &Path) -> Result<Self> {
        if path.exists() {
            let data = std::fs::read_to_string(path)?;
            let trusted: HashMap<String, TrustedPluginRecord> = serde_json::from_str(&data).unwrap_or_default();
            Ok(Self {
                path: path.to_path_buf(),
                trusted,
            })
        } else {
            let store = Self {
                path: path.to_path_buf(),
                trusted: HashMap::new(),
            };
            store.save()?;
            Ok(store)
        }
    }

    pub fn is_trusted(&self, name: &str, content: &str) -> bool {
        if let Some(record) = self.trusted.get(name) {
            let hash = Self::compute_hash(content);
            record.content_hash == hash
        } else {
            false
        }
    }

    pub fn trust_plugin(&mut self, name: &str, plugin_path: &Path, content: &str) -> Result<()> {
        let hash = Self::compute_hash(content);
        let record = TrustedPluginRecord {
            name: name.to_string(),
            path: plugin_path.to_string_lossy().to_string(),
            trusted_at: chrono::Utc::now().timestamp(),
            content_hash: hash,
        };
        self.trusted.insert(name.to_string(), record);
        self.save()
    }

    pub fn revoke(&mut self, name: &str) -> Result<()> {
        self.trusted.remove(name);
        self.save()
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.trusted)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }

    fn compute_hash(content: &str) -> String {
        // Simple 64-bit deterministic hash representation for content
        let mut hasher: u64 = 0xcbf29ce484222325;
        for byte in content.as_bytes() {
            hasher ^= *byte as u64;
            hasher = hasher.wrapping_mul(0x100000001b3);
        }
        format!("{:016x}", hasher)
    }
}
