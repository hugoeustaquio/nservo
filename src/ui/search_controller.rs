use std::cell::RefCell;

use crate::core::{SearchEvent, SearchState, format_search_status};
use crate::ui::status_bar::StatusBar;
use crate::ui::tab_view::TabView;

/// Coordinates Vim-style in-page search (`/`, `n`, `N`, `:noh`) across tabs.
pub struct SearchController {
    state: RefCell<SearchState>,
}

impl SearchController {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(SearchState::new()),
        }
    }

    /// Run a newly submitted `/` search (or repeat the previous search if empty).
    pub fn submit(&self, tab_view: &TabView, status_bar: &StatusBar, raw: &str) {
        let Some(query) = self.state.borrow_mut().submit(raw) else {
            return;
        };
        if let Some(tab) = tab_view.active_tab() {
            tab.search_page(&query);
        } else {
            status_bar.show_error("No active tab to search");
        }
    }

    /// Step to the next (`delta > 0`) or previous (`delta < 0`) match.
    pub fn step(&self, tab_view: &TabView, status_bar: &StatusBar, delta: i32) {
        let query = {
            let mut state = self.state.borrow_mut();
            let Some(q) = state.last_query().map(str::to_string) else {
                status_bar.show_notice("No previous search pattern");
                return;
            };
            state.mark_active();
            q
        };
        if let Some(tab) = tab_view.active_tab() {
            tab.search_step(delta, &query);
        }
    }

    /// Clear active highlights in the current tab. Returns `true` if highlights were cleared.
    pub fn clear(&self, tab_view: &TabView) -> bool {
        let was_active = self.state.borrow_mut().clear_highlights();
        if was_active && let Some(tab) = tab_view.active_tab() {
            tab.clear_search();
        }
        was_active
    }

    /// Display search match feedback reported by the active tab's JS bridge.
    pub fn handle_event(&self, status_bar: &StatusBar, event: SearchEvent) {
        match event {
            SearchEvent::Results {
                query,
                current,
                total,
                wrapped,
            } => {
                let (msg, is_err) = format_search_status(&query, current, total, wrapped);
                if is_err {
                    status_bar.show_error(&msg);
                } else {
                    status_bar.show_notice(&msg);
                }
            }
            SearchEvent::Cleared => {}
        }
    }
}

impl Default for SearchController {
    fn default() -> Self {
        Self::new()
    }
}
