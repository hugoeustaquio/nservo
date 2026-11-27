use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::{BrowserAction, Mode, default_config_path};

/// Metadata describing one customizable Normal-mode keyboard shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutSpec {
    pub id: &'static str,
    pub default_key: &'static str,
    pub category: &'static str,
    pub description: &'static str,
}

/// Canonical registry of all customizable Normal-mode shortcuts.
pub const SHORTCUT_SPECS: &[ShortcutSpec] = &[
    ShortcutSpec { id: "scroll_down", default_key: "j", category: "Navigation", description: "Scroll down (supports numeric count)" },
    ShortcutSpec { id: "scroll_up", default_key: "k", category: "Navigation", description: "Scroll up (supports numeric count)" },
    ShortcutSpec { id: "scroll_left", default_key: "h", category: "Navigation", description: "Scroll left (supports numeric count)" },
    ShortcutSpec { id: "scroll_right", default_key: "l", category: "Navigation", description: "Scroll right (supports numeric count)" },
    ShortcutSpec { id: "scroll_top", default_key: "gg", category: "Navigation", description: "Scroll to top of page" },
    ShortcutSpec { id: "scroll_bottom", default_key: "G", category: "Navigation", description: "Scroll to bottom of page" },
    ShortcutSpec { id: "go_back", default_key: "H", category: "Navigation", description: "Navigate back in history" },
    ShortcutSpec { id: "go_forward", default_key: "L", category: "Navigation", description: "Navigate forward in history" },
    ShortcutSpec { id: "reload", default_key: "r", category: "Navigation", description: "Reload current tab" },
    ShortcutSpec { id: "next_tab", default_key: "gt", category: "Tabs", description: "Switch to next tab" },
    ShortcutSpec { id: "prev_tab", default_key: "gT", category: "Tabs", description: "Switch to previous tab" },
    ShortcutSpec { id: "close_tab", default_key: "d", category: "Tabs", description: "Close current tab" },
    ShortcutSpec { id: "reopen_tab", default_key: "u", category: "Tabs", description: "Reopen last closed tab" },
    ShortcutSpec { id: "open_prompt", default_key: "o", category: "Commands & Modes", description: "Open command bar with :open" },
    ShortcutSpec { id: "open_new_tab_prompt", default_key: "O", category: "Commands & Modes", description: "Open command bar with :Open" },
    ShortcutSpec { id: "edit_url", default_key: "go", category: "Commands & Modes", description: "Edit current tab URL in command bar" },
    ShortcutSpec { id: "command_mode", default_key: ":", category: "Commands & Modes", description: "Enter Command mode" },
    ShortcutSpec { id: "insert_mode", default_key: "i", category: "Commands & Modes", description: "Enter Insert mode" },
    ShortcutSpec { id: "hints", default_key: "f", category: "Hints & Search", description: "Follow links/inputs with hints" },
    ShortcutSpec { id: "hints_new_tab", default_key: "F", category: "Hints & Search", description: "Open link in new tab with hints" },
    ShortcutSpec { id: "search", default_key: "/", category: "Hints & Search", description: "Search text in current page" },
    ShortcutSpec { id: "search_next", default_key: "n", category: "Hints & Search", description: "Jump to next search match" },
    ShortcutSpec { id: "search_prev", default_key: "N", category: "Hints & Search", description: "Jump to previous search match" },
    ShortcutSpec { id: "add_bookmark", default_key: "b", category: "Library & Tools", description: "Bookmark current page" },
    ShortcutSpec { id: "show_bookmarks", default_key: "B", category: "Library & Tools", description: "Open bookmarks page (about:bookmarks)" },
    ShortcutSpec { id: "show_history", default_key: "gh", category: "Library & Tools", description: "Open browsing history (about:history)" },
    ShortcutSpec { id: "toggle_inspector", default_key: "wi", category: "Library & Tools", description: "Toggle Web Inspector drawer" },
];

/// Persistent user configuration stored at `$XDG_CONFIG_HOME/nservo/config.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserConfig {
    #[serde(default)]
    pub keybindings: BTreeMap<String, String>,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        let mut keybindings = BTreeMap::new();
        for spec in SHORTCUT_SPECS {
            keybindings.insert(spec.id.to_string(), spec.default_key.to_string());
        }
        Self { keybindings }
    }
}

impl BrowserConfig {
    /// Load configuration from `$XDG_CONFIG_HOME/nservo/config.json`, falling
    /// back cleanly to defaults if missing or invalid.
    pub fn load_default() -> Self {
        Self::load(&default_config_path())
    }

    /// Load configuration from `path`, filling any missing keys from defaults.
    pub fn load(path: &Path) -> Self {
        let Ok(raw) = fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(parsed) = serde_json::from_str::<BrowserConfig>(&raw) else {
            return Self::default();
        };
        let merged = Self::merge_with_defaults(parsed.keybindings);
        if merged.validate().is_ok() {
            merged
        } else {
            Self::default()
        }
    }

    /// Merge partial user keybindings onto defaults, dropping unknown action IDs.
    pub fn merge_with_defaults(overrides: BTreeMap<String, String>) -> Self {
        let mut cfg = Self::default();
        for spec in SHORTCUT_SPECS {
            if let Some(val) = overrides.get(spec.id) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    cfg.keybindings.insert(spec.id.to_string(), trimmed.to_string());
                }
            }
        }
        cfg
    }

    /// Return the configured key sequence for `action_id`.
    pub fn key_for(&self, action_id: &str) -> &str {
        self.keybindings
            .get(action_id)
            .map(String::as_str)
            .or_else(|| SHORTCUT_SPECS.iter().find(|s| s.id == action_id).map(|s| s.default_key))
            .unwrap_or("")
    }

    /// Validate that all keybindings are 1- or 2-character sequences, do not start
    /// with count digits `'1'..='9'`, and do not collide or shadow each other.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen: HashMap<&str, &str> = HashMap::new();
        let mut single_chars: HashMap<char, &str> = HashMap::new();
        let mut prefixes: HashMap<char, &str> = HashMap::new();

        for spec in SHORTCUT_SPECS {
            let seq = self.key_for(spec.id);
            let chars: Vec<char> = seq.chars().collect();
            if chars.is_empty() || chars.len() > 2 {
                return Err(format!("'{}' must be 1 or 2 characters (got '{seq}')", spec.id));
            }
            if chars.iter().any(|c| c.is_whitespace() || c.is_control()) {
                return Err(format!("'{}' cannot contain whitespace or control chars", spec.id));
            }
            if matches!(chars[0], '1'..='9') {
                return Err(format!("'{}' cannot start with digit '{}' (reserved for counts)", spec.id, chars[0]));
            }
            if let Some(other) = seen.insert(seq, spec.id) {
                return Err(format!("Duplicate shortcut '{seq}' assigned to '{other}' and '{}'", spec.id));
            }
            if chars.len() == 1 {
                single_chars.insert(chars[0], spec.id);
            } else {
                prefixes.insert(chars[0], spec.id);
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
        for seq in self.keybindings.values() {
            let mut chars = seq.chars();
            if let (Some(first), Some(_), None) = (chars.next(), chars.next(), chars.next()) {
                out.insert(first);
            }
        }
        out
    }

    /// Resolve a 1- or 2-character Normal-mode sequence to a [`BrowserAction`].
    pub fn resolve_action(&self, seq: &str, count: u32) -> Option<BrowserAction> {
        let id = self
            .keybindings
            .iter()
            .find(|(_, k)| k.as_str() == seq)
            .map(|(id, _)| id.as_str())?;
        action_for_id(id, count)
    }
}

fn action_for_id(id: &str, count: u32) -> Option<BrowserAction> {
    Some(match id {
        "scroll_down" => BrowserAction::ScrollDown(count),
        "scroll_up" => BrowserAction::ScrollUp(count),
        "scroll_left" => BrowserAction::ScrollLeft(count),
        "scroll_right" => BrowserAction::ScrollRight(count),
        "scroll_top" => BrowserAction::ScrollTop,
        "scroll_bottom" => BrowserAction::ScrollBottom,
        "go_back" => BrowserAction::GoBack,
        "go_forward" => BrowserAction::GoForward,
        "reload" => BrowserAction::Reload,
        "next_tab" => BrowserAction::NextTab,
        "prev_tab" => BrowserAction::PrevTab,
        "close_tab" => BrowserAction::CloseTab,
        "reopen_tab" => BrowserAction::ReopenClosedTab,
        "open_prompt" => BrowserAction::EnterCommandWith("open ".to_string()),
        "open_new_tab_prompt" => BrowserAction::EnterCommandWith("Open ".to_string()),
        "edit_url" => BrowserAction::EditCurrentUrl,
        "command_mode" => BrowserAction::EnterMode(Mode::Command),
        "insert_mode" => BrowserAction::EnterMode(Mode::Insert),
        "hints" => BrowserAction::StartHints { new_tab: false },
        "hints_new_tab" => BrowserAction::StartHints { new_tab: true },
        "search" => BrowserAction::EnterMode(Mode::Search),
        "search_next" => BrowserAction::SearchNext,
        "search_prev" => BrowserAction::SearchPrev,
        "add_bookmark" => BrowserAction::AddBookmark,
        "show_bookmarks" => BrowserAction::ShowBookmarks,
        "show_history" => BrowserAction::ShowHistory(None),
        "toggle_inspector" => BrowserAction::ToggleInspector,
        _ => return None,
    })
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
            Some(ConfigEvent::Save(BrowserConfig::merge_with_defaults(overrides)))
        }
        _ => None,
    }
}
