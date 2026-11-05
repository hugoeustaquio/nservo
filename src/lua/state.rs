use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::bookmarks::BookmarksDatabase;
use crate::config::{ConfigOptions, KeybindingMap};
use crate::history::HistoryDatabase;

/// Shared thread-safe state passed between the Rust engine and Lua runtime.
pub struct BrowserSharedState {
    pub current_url: String,
    pub current_title: String,
    pub current_html: String,
    pub pending_js_eval: Option<String>,
    pub notifications: Vec<String>,
    pub keybindings: KeybindingMap,
    pub options: ConfigOptions,
    pub history: Option<Arc<Mutex<HistoryDatabase>>>,
    pub bookmarks: Option<Arc<Mutex<BookmarksDatabase>>>,
    pub tab_actions: Vec<String>,
    pub custom_commands: HashMap<String, mlua::RegistryKey>,
}

impl BrowserSharedState {
    pub fn new() -> Self {
        Self {
            current_url: "about:blank".to_string(),
            current_title: "NServo".to_string(),
            current_html: String::new(),
            pending_js_eval: None,
            notifications: Vec::new(),
            keybindings: KeybindingMap::new(),
            options: ConfigOptions::new(),
            history: None,
            bookmarks: None,
            tab_actions: Vec::new(),
            custom_commands: HashMap::new(),
        }
    }
}

impl Default for BrowserSharedState {
    fn default() -> Self {
        Self::new()
    }
}
