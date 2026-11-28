use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::config_specs::action_for_id;
pub use crate::core::config_specs::{SHORTCUT_SPECS, ShortcutSpec};
use crate::core::plugins::InstalledPlugin;
use crate::core::{BrowserAction, default_config_path};

/// Default `User-Agent` header sent by `nservo` unless overridden in `config.json`.
pub const DEFAULT_USER_AGENT: &str =
    concat!("NServo ", env!("CARGO_PKG_VERSION"), " powered by Servo");

fn default_user_agent() -> String {
    DEFAULT_USER_AGENT.to_string()
}

/// Persistent user configuration stored at `$XDG_CONFIG_HOME/nservo/config.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserConfig {
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    #[serde(default)]
    pub keybindings: BTreeMap<String, String>,
    #[serde(default)]
    pub plugin_keybindings: BTreeMap<String, String>,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        let mut keybindings = BTreeMap::new();
        for spec in SHORTCUT_SPECS {
            keybindings.insert(spec.id.to_string(), spec.default_key.to_string());
        }
        Self {
            user_agent: default_user_agent(),
            keybindings,
            plugin_keybindings: BTreeMap::new(),
        }
    }
}

impl BrowserConfig {
    pub fn load_default() -> Self {
        Self::load(&default_config_path())
    }

    pub fn load(path: &Path) -> Self {
        let Ok(raw) = fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(parsed) = serde_json::from_str::<BrowserConfig>(&raw) else {
            return Self::default();
        };
        let merged = Self::merge_with_defaults(parsed.keybindings, parsed.plugin_keybindings)
            .with_user_agent(Some(&parsed.user_agent));
        if merged.validate().is_ok() {
            merged
        } else {
            Self::default()
        }
    }

    /// Apply an optional user-agent override, falling back to [`DEFAULT_USER_AGENT`] if blank.
    pub fn with_user_agent(mut self, ua: Option<&str>) -> Self {
        let trimmed = ua.map(str::trim).unwrap_or("");
        self.user_agent = if trimmed.is_empty() {
            default_user_agent()
        } else {
            trimmed.to_string()
        };
        self
    }

    /// Merge partial user keybindings onto defaults, preserving valid plugin bindings.
    pub fn merge_with_defaults(
        overrides: BTreeMap<String, String>,
        plugin_overrides: BTreeMap<String, String>,
    ) -> Self {
        let mut cfg = Self::default();
        for spec in SHORTCUT_SPECS {
            if let Some(val) = overrides.get(spec.id) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    cfg.keybindings
                        .insert(spec.id.to_string(), trimmed.to_string());
                }
            }
        }
        for (name, val) in plugin_overrides {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                cfg.plugin_keybindings.insert(name, trimmed.to_string());
            }
        }
        cfg
    }

    /// Fill missing `plugin_keybindings` from installed plugins' `default_shortcut`
    /// only when the default does not conflict with existing bindings.
    pub fn apply_plugin_defaults(&mut self, plugins: &[InstalledPlugin]) {
        for p in plugins {
            if self.plugin_keybindings.contains_key(&p.manifest.name) {
                continue;
            }
            if let Some(def) = p.manifest.default_shortcut.as_deref() {
                let trimmed = def.trim();
                if trimmed.is_empty() {
                    continue;
                }
                self.plugin_keybindings
                    .insert(p.manifest.name.clone(), trimmed.to_string());
                if self.validate().is_err() {
                    self.plugin_keybindings.remove(&p.manifest.name);
                }
            }
        }
    }

    pub fn key_for(&self, action_id: &str) -> &str {
        self.keybindings
            .get(action_id)
            .map(String::as_str)
            .or_else(|| {
                SHORTCUT_SPECS
                    .iter()
                    .find(|s| s.id == action_id)
                    .map(|s| s.default_key)
            })
            .unwrap_or("")
    }

    pub fn plugin_key_for(&self, name: &str) -> &str {
        self.plugin_keybindings
            .get(name)
            .map(String::as_str)
            .unwrap_or("")
    }

    /// Validate user-agent and ensure all built-in/plugin keybindings are non-conflicting.
    pub fn validate(&self) -> Result<(), String> {
        let ua = self.user_agent.trim();
        if ua.is_empty() || !ua.bytes().all(|b| b == b'\t' || (0x20..=0x7e).contains(&b)) {
            return Err("User-Agent must be non-empty printable ASCII".to_string());
        }
        let mut seen: HashMap<&str, String> = HashMap::new();
        let mut single_chars: HashMap<char, String> = HashMap::new();
        let mut prefixes: HashMap<char, String> = HashMap::new();
        let all_entries = SHORTCUT_SPECS
            .iter()
            .map(|s| (s.id.to_string(), self.key_for(s.id)))
            .chain(
                self.plugin_keybindings
                    .iter()
                    .map(|(k, v)| (format!("plugin:{k}"), v.as_str())),
            );

        for (label, seq) in all_entries {
            let chars: Vec<char> = seq.chars().collect();
            if chars.is_empty() || chars.len() > 2 {
                return Err(format!("'{label}' must be 1 or 2 characters (got '{seq}')"));
            }
            if chars.iter().any(|c| c.is_whitespace() || c.is_control()) {
                return Err(format!(
                    "'{label}' cannot contain whitespace or control chars"
                ));
            }
            if matches!(chars[0], '1'..='9') {
                return Err(format!(
                    "'{label}' cannot start with digit '{}' (reserved for counts)",
                    chars[0]
                ));
            }
            if let Some(other) = seen.insert(seq, label.clone()) {
                return Err(format!(
                    "Duplicate shortcut '{seq}' assigned to '{other}' and '{label}'"
                ));
            }
            if chars.len() == 1 {
                single_chars.insert(chars[0], label);
            } else {
                prefixes.insert(chars[0], label);
            }
        }
        for (ch, single_id) in &single_chars {
            if let Some(seq_id) = prefixes.get(ch) {
                return Err(format!(
                    "Shortcut '{ch}' ('{single_id}') conflicts with prefix of '{seq_id}'"
                ));
            }
        }
        Ok(())
    }

    /// Persist this configuration to `path` as pretty-printed JSON.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {}: {e}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {e}"))?;
        let tmp = path.with_extension(format!("json.tmp.{}", std::process::id()));
        fs::write(&tmp, format!("{json}\n"))
            .map_err(|e| format!("Failed to write {}: {e}", tmp.display()))?;
        fs::rename(&tmp, path).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            format!("Failed to save {}: {e}", path.display())
        })
    }

    /// Return the set of first characters that begin any 2-key sequence.
    pub fn sequence_prefixes(&self) -> HashSet<char> {
        let mut out = HashSet::new();
        for seq in self
            .keybindings
            .values()
            .chain(self.plugin_keybindings.values())
        {
            let mut chars = seq.chars();
            if let (Some(first), Some(_), None) = (chars.next(), chars.next(), chars.next()) {
                out.insert(first);
            }
        }
        out
    }

    /// Resolve a 1- or 2-character Normal-mode sequence to a [`BrowserAction`].
    pub fn resolve_action(&self, seq: &str, count: u32) -> Option<BrowserAction> {
        if let Some((id, _)) = self.keybindings.iter().find(|(_, k)| k.as_str() == seq) {
            return action_for_id(id, count);
        }
        self.plugin_keybindings
            .iter()
            .find(|(_, k)| k.as_str() == seq)
            .map(|(name, _)| BrowserAction::RunPlugin {
                name: name.clone(),
                args: String::new(),
            })
    }
}

/// Messages posted from the built-in `about:config` page back to `nservo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigEvent {
    Save(BrowserConfig),
    Reset,
}

/// Parse a JSON message sent by `about:config` via `nservoConfig`.
pub fn parse_config_message(body: &str) -> Option<ConfigEvent> {
    let val: Value = serde_json::from_str(body).ok()?;
    match val.get("action")?.as_str()? {
        "reset" => Some(ConfigEvent::Reset),
        "save" => {
            let map = val.get("keybindings")?.as_object()?;
            let mut overrides = BTreeMap::new();
            for (k, v) in map {
                if let Some(s) = v.as_str() {
                    overrides.insert(k.clone(), s.to_string());
                }
            }
            let mut plug = BTreeMap::new();
            if let Some(pmap) = val.get("plugin_keybindings").and_then(Value::as_object) {
                for (k, v) in pmap {
                    if let Some(s) = v.as_str() {
                        plug.insert(k.clone(), s.to_string());
                    }
                }
            }
            let ua = val.get("user_agent").and_then(Value::as_str);
            Some(ConfigEvent::Save(
                BrowserConfig::merge_with_defaults(overrides, plug).with_user_agent(ua),
            ))
        }
        _ => None,
    }
}
