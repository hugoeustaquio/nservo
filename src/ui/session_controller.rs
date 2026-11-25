use crate::core::{DEFAULT_SESSION_NAME, Session, SessionStore, normalize_url};
use crate::engine::start_page;
use crate::ui::status_bar::StatusBar;
use crate::ui::tab_view::TabView;

/// Bridges [`SessionStore`] persistence with [`TabView`] and [`StatusBar`].
pub struct SessionController {
    store: SessionStore,
}

impl SessionController {
    pub fn new() -> Self {
        Self {
            store: SessionStore::default_store(),
        }
    }

    pub fn open_initial_tabs(&self, tab_view: &TabView, initial_url: Option<&str>) {
        if let Some(url) = initial_url
            && !url.trim().is_empty()
        {
            tab_view.open_url_in_new_tab(&normalize_url(url));
            return;
        }
        if let Ok(Some(session)) = self.store.load_optional(DEFAULT_SESSION_NAME)
            && tab_view.restore_session(&session)
        {
            return;
        }
        tab_view.open_html_in_new_tab("nservo — Help", start_page::HELP_PAGE_HTML);
    }

    pub fn save(&self, tab_view: &TabView, status_bar: &StatusBar, name: &str) -> bool {
        let (tabs, active_index, _) = tab_view.snapshot();
        let session = Session::from_tabs(name, &tabs, active_index);
        match self.store.save(&session) {
            Ok(()) => {
                status_bar.show_notice(&format!("Saved session '{name}'"));
                true
            }
            Err(err) => {
                status_bar.show_error(&err.to_string());
                false
            }
        }
    }

    pub fn load(&self, tab_view: &TabView, status_bar: &StatusBar, name: &str) {
        match self.store.load(name) {
            Ok(session) if tab_view.restore_session(&session) => {
                status_bar.show_notice(&format!("Restored session '{name}'"));
            }
            Ok(_) => status_bar.show_error(&format!("Session '{name}' has no tabs")),
            Err(err) => status_bar.show_error(&err.to_string()),
        }
    }
}

impl Default for SessionController {
    fn default() -> Self {
        Self::new()
    }
}
