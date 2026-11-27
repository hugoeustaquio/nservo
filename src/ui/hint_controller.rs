use std::cell::RefCell;

use crate::ui::tab_view::TabView;

/// Tracks the currently typed hint prefix while in Hint mode (`f`).
#[derive(Default)]
pub struct HintController {
    typed: RefCell<String>,
}

impl HintController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&self, tab_view: &TabView, new_tab: bool) {
        self.typed.borrow_mut().clear();
        if let Some(tab) = tab_view.active_tab() {
            tab.start_hints(new_tab);
        }
    }

    pub fn push_char(&self, tab_view: &TabView, ch: char) -> String {
        let current = {
            let mut typed = self.typed.borrow_mut();
            typed.push(ch);
            typed.clone()
        };
        if let Some(tab) = tab_view.active_tab() {
            tab.filter_hints(&current);
        }
        current
    }

    /// Delete the last typed hint character. Returns `None` if already empty
    /// (signaling Hint mode should exit).
    pub fn pop_char(&self, tab_view: &TabView) -> Option<String> {
        let remaining = {
            let mut typed = self.typed.borrow_mut();
            typed.pop()?;
            typed.clone()
        };
        if let Some(tab) = tab_view.active_tab() {
            tab.filter_hints(&remaining);
        }
        Some(remaining)
    }

    pub fn clear(&self, tab_view: &TabView) {
        self.typed.borrow_mut().clear();
        if let Some(tab) = tab_view.active_tab() {
            tab.clear_hints();
        }
    }
}
