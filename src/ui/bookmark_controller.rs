use crate::core::{AddBookmarkOutcome, BookmarkStore};
use crate::engine::bookmarks_page::{BOOKMARKS_PAGE_URL, render_bookmarks_html};
use crate::ui::status_bar::StatusBar;
use crate::ui::tab_view::TabView;

/// Bridges [`BookmarkStore`] persistence with [`TabView`] and [`StatusBar`].
pub struct BookmarkController {
    store: BookmarkStore,
}

impl BookmarkController {
    pub fn new() -> Self {
        Self {
            store: BookmarkStore::default_store(),
        }
    }

    /// Bookmark the currently active tab (`b` or `:bookmark`).
    pub fn add_current(&self, tab_view: &TabView, status_bar: &StatusBar) {
        let Some(tab) = tab_view.active_tab_info() else {
            status_bar.show_error("No active tab to bookmark");
            return;
        };
        match self.store.add(&tab.url, &tab.title) {
            Ok(AddBookmarkOutcome::Added(bm)) => {
                status_bar.show_notice(&format!("Bookmarked: {}", bm.display_title()));
            }
            Ok(AddBookmarkOutcome::AlreadyExists(bm)) => {
                status_bar.show_notice(&format!("Already bookmarked: {}", bm.display_title()));
            }
            Err(err) => status_bar.show_error(&err.to_string()),
        }
    }

    /// Render and open the built-in `about:bookmarks` page in the active tab (`B` or `:bookmarks`).
    pub fn show(&self, tab_view: &TabView, status_bar: &StatusBar) {
        match self.store.list() {
            Ok(bookmarks) => {
                let html = render_bookmarks_html(&bookmarks);
                tab_view.load_html_in_active_tab(BOOKMARKS_PAGE_URL, "nservo — Bookmarks", &html);
            }
            Err(err) => status_bar.show_error(&err.to_string()),
        }
    }

    /// Remove a bookmark by 1-based index or URL (defaults to the active tab's URL).
    pub fn remove(&self, tab_view: &TabView, status_bar: &StatusBar, target: Option<&str>) {
        let lookup = match target {
            Some(arg) => arg.to_string(),
            None => match tab_view.active_url() {
                Some(url) if !url.is_empty() => url,
                _ => {
                    status_bar.show_error("No active page URL to remove from bookmarks");
                    return;
                }
            },
        };

        let viewing_bookmarks = tab_view.active_url().as_deref() == Some(BOOKMARKS_PAGE_URL);
        match self.store.remove(&lookup) {
            Ok(Some(removed)) => {
                if viewing_bookmarks {
                    self.show(tab_view, status_bar);
                }
                status_bar.show_notice(&format!("Removed bookmark: {}", removed.display_title()));
            }
            Ok(None) => status_bar.show_error(&format!("Bookmark not found: {lookup}")),
            Err(err) => status_bar.show_error(&err.to_string()),
        }
    }
}

impl Default for BookmarkController {
    fn default() -> Self {
        Self::new()
    }
}
