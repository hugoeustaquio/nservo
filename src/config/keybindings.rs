use std::collections::HashMap;

/// Maps key combinations (in Normal Mode) to browser actions.
#[derive(Debug, Clone)]
pub struct KeybindingMap {
    bindings: HashMap<String, String>,
}

impl KeybindingMap {
    pub fn new() -> Self {
        let mut map = Self {
            bindings: HashMap::new(),
        };
        map.populate_defaults();
        map
    }

    fn populate_defaults(&mut self) {
        self.bind("j", "scroll_down");
        self.bind("k", "scroll_up");
        self.bind("h", "scroll_left");
        self.bind("l", "scroll_right");
        self.bind("o", "open_url_current_tab");
        self.bind("O", "open_url_new_tab");
        self.bind("d", "close_tab");
        self.bind("i", "enter_insert_mode");
        self.bind("ESC", "exit_insert_mode");
        self.bind(":", "enter_command_mode");
    }

    pub fn bind(&mut self, key: &str, action: &str) {
        self.bindings.insert(key.to_string(), action.to_string());
    }

    pub fn get_action(&self, key: &str) -> Option<&String> {
        self.bindings.get(key)
    }

    pub fn unbind(&mut self, key: &str) {
        self.bindings.remove(key);
    }

    pub fn all_bindings(&self) -> &HashMap<String, String> {
        &self.bindings
    }
}

impl Default for KeybindingMap {
    fn default() -> Self {
        Self::new()
    }
}
