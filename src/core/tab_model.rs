/// Metadata tracked for each open browser tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabInfo {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub is_loading: bool,
}

impl TabInfo {
    pub fn display_title(&self) -> &str {
        if !self.title.is_empty() {
            &self.title
        } else if !self.url.is_empty() {
            &self.url
        } else {
            "New Tab"
        }
    }
}

/// Pure, UI-independent state for the ordered list of browser tabs.
#[derive(Debug, Default)]
pub struct TabModel {
    tabs: Vec<TabInfo>,
    active: usize,
    next_id: u64,
}

impl TabModel {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active: 0,
            next_id: 1,
        }
    }

    pub fn tabs(&self) -> &[TabInfo] {
        &self.tabs
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn active_tab(&self) -> Option<&TabInfo> {
        self.tabs.get(self.active)
    }

    /// Add a new tab and make it active, returning its unique tab ID.
    pub fn add_tab(&mut self, url: impl Into<String>, title: impl Into<String>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(TabInfo {
            id,
            url: url.into(),
            title: title.into(),
            is_loading: false,
        });
        self.active = self.tabs.len() - 1;
        id
    }

    /// Select a tab by zero-based index if within bounds.
    pub fn select_index(&mut self, index: usize) -> Option<u64> {
        if index < self.tabs.len() {
            self.active = index;
            Some(self.tabs[index].id)
        } else {
            None
        }
    }

    /// Switch to the next tab, wrapping from the last tab back to the first (`gt`).
    pub fn next_tab(&mut self) -> Option<u64> {
        if self.tabs.is_empty() {
            return None;
        }
        self.active = (self.active + 1) % self.tabs.len();
        Some(self.tabs[self.active].id)
    }

    /// Switch to the previous tab, wrapping from the first tab to the last (`gT`).
    pub fn prev_tab(&mut self) -> Option<u64> {
        if self.tabs.is_empty() {
            return None;
        }
        self.active = if self.active == 0 {
            self.tabs.len() - 1
        } else {
            self.active - 1
        };
        Some(self.tabs[self.active].id)
    }

    /// Close the currently active tab and return its removed ID.
    pub fn close_active(&mut self) -> Option<u64> {
        if self.tabs.is_empty() {
            return None;
        }
        let removed = self.tabs.remove(self.active);
        if self.active >= self.tabs.len() && !self.tabs.is_empty() {
            self.active = self.tabs.len() - 1;
        }
        Some(removed.id)
    }

    pub fn update_url(&mut self, id: u64, url: String) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.url = url;
        }
    }

    pub fn update_title(&mut self, id: u64, title: String) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.title = title;
        }
    }

    pub fn set_loading(&mut self, id: u64, is_loading: bool) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.is_loading = is_loading;
        }
    }
}
