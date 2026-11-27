use crate::core::{HistoryStore, HistorySync};
use crate::engine::history_page::{HISTORY_PAGE_URL, render_history_html};
use crate::ui::status_bar::StatusBar;
use crate::ui::tab_view::TabView;

const HISTORY_PAGE_LIMIT: usize = 200;

/// Bridges SQLite [`HistoryStore`] persistence with [`TabView`] and [`StatusBar`].
pub struct HistoryController {
    store: HistoryStore,
}

impl HistoryController {
    pub fn new() -> Self {
        Self {
            store: HistoryStore::default_store(),
        }
    }

    /// Record a visit or update a visited page's title in SQLite.
    pub fn handle_sync(&self, sync: HistorySync) {
        let result = match sync {
            HistorySync::Visit { url, title } => self.store.record_visit(&url, &title),
            HistorySync::Title { url, title } => self.store.update_title(&url, &title),
        };
        if let Err(err) = result {
            log::warn!("Failed to persist browsing history: {err}");
        }
    }

    /// Render and open `about:history` (optionally filtered by `query`) in the active tab.
    pub fn show(&self, tab_view: &TabView, status_bar: &StatusBar, query: Option<&str>) {
        let entries = match query.map(str::trim).filter(|q| !q.is_empty()) {
            Some(q) => self.store.search(q, HISTORY_PAGE_LIMIT),
            None => self.store.recent(HISTORY_PAGE_LIMIT),
        };
        match entries {
            Ok(list) => {
                let html = render_history_html(&list, query);
                tab_view.load_html_in_active_tab(HISTORY_PAGE_URL, "nservo — History", &html);
            }
            Err(err) => status_bar.show_error(&err.to_string()),
        }
    }

    /// Delete all saved history entries and refresh `about:history` if currently open.
    pub fn clear(&self, tab_view: &TabView, status_bar: &StatusBar) {
        let viewing_history = tab_view.active_url().as_deref() == Some(HISTORY_PAGE_URL);
        match self.store.clear() {
            Ok(removed) => {
                if viewing_history {
                    self.show(tab_view, status_bar, None);
                }
                status_bar.show_notice(&format!("Cleared {removed} history entries"));
            }
            Err(err) => status_bar.show_error(&err.to_string()),
        }
    }
}

impl Default for HistoryController {
    fn default() -> Self {
        Self::new()
    }
}
