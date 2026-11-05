use crate::error::{NServoError, Result};
use crate::tabs::tab::Tab;

pub struct TabManager {
    tabs: Vec<Tab>,
    active_index: usize,
    next_id: usize,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_index: 0,
            next_id: 1,
        }
    }

    pub fn open_tab(&mut self, url: &str) -> Result<usize> {
        let id = self.next_id;
        self.next_id += 1;
        let tab = Tab::new(id, url)?;
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
        Ok(self.active_index)
    }

    pub fn open_mock_tab(&mut self, url: &str) -> Result<usize> {
        let id = self.next_id;
        self.next_id += 1;
        let tab = Tab::new_mock(id, url)?;
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
        Ok(self.active_index)
    }

    pub fn close_current(&mut self) -> Result<()> {
        if self.tabs.is_empty() {
            return Ok(());
        }
        self.tabs.remove(self.active_index);
        if self.active_index >= self.tabs.len() && !self.tabs.is_empty() {
            self.active_index = self.tabs.len() - 1;
        }
        Ok(())
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_index = (self.active_index + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            if self.active_index == 0 {
                self.active_index = self.tabs.len() - 1;
            } else {
                self.active_index -= 1;
            }
        }
    }

    pub fn current_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_index)
    }

    pub fn current_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_index)
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn active_index(&self) -> usize {
        self.active_index
    }

    pub fn set_active_index(&mut self, index: usize) -> Result<()> {
        if index < self.tabs.len() {
            self.active_index = index;
            Ok(())
        } else {
            Err(NServoError::Config("Tab index out of range".to_string()))
        }
    }
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}
