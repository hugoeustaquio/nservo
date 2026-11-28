use std::rc::Rc;

use crate::core::{
    BookmarkStore, BrowserConfig, ConfigEvent, HintEvent, HistoryStore, HistorySync,
    InspectorEvent, SearchEvent,
};
use crate::engine::bookmarks_page::{BOOKMARKS_PAGE_URL, render_bookmarks_html};
use crate::engine::config_page::{CONFIG_PAGE_URL, render_default_config_html};
use crate::engine::history_page::{HISTORY_PAGE_URL, render_history_html};
use crate::engine::new_tab_page::{START_PAGE_URL, load_start_page_html};
use crate::engine::start_page;
use crate::engine::web_tab::WebTab;
use crate::ui::page_overlay::PageOverlay;

#[derive(Clone)]
pub(super) struct TabEntry {
    pub(super) tab: WebTab,
    pub(super) overlay: PageOverlay,
}

/// Callbacks emitted when tab metadata changes, navigation occurs, popups open, or page bridges report.
#[derive(Clone)]
pub struct TabViewCallbacks {
    pub on_state_changed: Rc<dyn Fn()>,
    pub on_popup_url: Rc<dyn Fn(String)>,
    pub on_hint_event: Rc<dyn Fn(u64, HintEvent)>,
    pub on_inspector_event: Rc<dyn Fn(u64, InspectorEvent)>,
    pub on_search_event: Rc<dyn Fn(u64, SearchEvent)>,
    pub on_config_event: Rc<dyn Fn(u64, ConfigEvent)>,
    pub on_history_sync: Rc<dyn Fn(HistorySync)>,
    pub on_navigation: Rc<dyn Fn(u64)>,
}

pub(super) fn config_page_if_requested(url: &str) -> Option<String> {
    (url == CONFIG_PAGE_URL).then(|| render_default_config_html(&BrowserConfig::load_default()))
}

pub(super) fn tab_stack_name(id: u64) -> String {
    format!("tab-{id}")
}

pub(super) fn web_tab_for_url(url: &str) -> (&str, WebTab) {
    if url == BOOKMARKS_PAGE_URL {
        let list = BookmarkStore::default_store().list().unwrap_or_default();
        (
            BOOKMARKS_PAGE_URL,
            WebTab::new_html(&render_bookmarks_html(&list)),
        )
    } else if url == HISTORY_PAGE_URL {
        let list = HistoryStore::default_store()
            .recent(200)
            .unwrap_or_default();
        (
            HISTORY_PAGE_URL,
            WebTab::new_html(&render_history_html(&list, None)),
        )
    } else if url == START_PAGE_URL {
        (START_PAGE_URL, WebTab::new_html(&load_start_page_html()))
    } else if url == CONFIG_PAGE_URL {
        let html = render_default_config_html(&BrowserConfig::load_default());
        (CONFIG_PAGE_URL, WebTab::new_html(&html))
    } else if url == "about:help" || url.trim().is_empty() {
        ("about:help", WebTab::new_html(start_page::HELP_PAGE_HTML))
    } else {
        (url, WebTab::new_url(url))
    }
}
