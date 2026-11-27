use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk::prelude::*;
use servo_gtk::WebView;

use crate::core::{
    BookmarkStore, HintEvent, HistoryStore, HistorySync, InspectorEvent, SearchEvent, Session,
    TabInfo, TabModel,
};
use crate::engine::bookmarks_page::{BOOKMARKS_PAGE_URL, render_bookmarks_html};
use crate::engine::history_page::{HISTORY_PAGE_URL, render_history_html};
use crate::engine::start_page;
use crate::engine::web_tab::WebTab;
use crate::ui::page_overlay::PageOverlay;
use crate::ui::tab_signals::wire_tab_signals;

#[derive(Clone)]
struct TabEntry {
    tab: WebTab,
    overlay: PageOverlay,
}

/// Callbacks emitted when tab metadata changes, navigation occurs, popups open, or page bridges report.
#[derive(Clone)]
pub struct TabViewCallbacks {
    pub on_state_changed: Rc<dyn Fn()>,
    pub on_popup_url: Rc<dyn Fn(String)>,
    pub on_hint_event: Rc<dyn Fn(u64, HintEvent)>,
    pub on_inspector_event: Rc<dyn Fn(u64, InspectorEvent)>,
    pub on_search_event: Rc<dyn Fn(u64, SearchEvent)>,
    pub on_history_sync: Rc<dyn Fn(HistorySync)>,
    pub on_navigation: Rc<dyn Fn(u64)>,
}

/// Hosts one [`WebTab`] per tab inside a [`gtk::Stack`].
#[derive(Clone)]
pub struct TabView {
    stack: gtk::Stack,
    model: Rc<RefCell<TabModel>>,
    tabs: Rc<RefCell<HashMap<u64, TabEntry>>>,
    callbacks: TabViewCallbacks,
}

impl TabView {
    pub fn new(callbacks: TabViewCallbacks) -> Self {
        let stack = gtk::Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);

        Self {
            stack,
            model: Rc::new(RefCell::new(TabModel::new())),
            tabs: Rc::new(RefCell::new(HashMap::new())),
            callbacks,
        }
    }

    pub fn widget(&self) -> &gtk::Stack {
        &self.stack
    }

    pub fn open_url_in_new_tab(&self, url: &str) {
        let id = self.insert_tab(url, "", WebTab::new_url(url));
        self.activate_tab_id(id);
    }

    pub fn open_html_in_new_tab(&self, url: &str, title: &str, html: &str) {
        let id = self.insert_tab(url, title, WebTab::new_html(html));
        self.activate_tab_id(id);
    }

    /// Replace all open tabs with the tabs stored in `session`.
    /// Returns `false` if `session` contains no tabs.
    pub fn restore_session(&self, session: &Session) -> bool {
        if session.tabs.is_empty() {
            return false;
        }

        let old_tabs: Vec<TabEntry> = self.tabs.borrow_mut().drain().map(|(_, t)| t).collect();
        for entry in old_tabs {
            self.stack.remove(entry.overlay.widget());
        }
        {
            let mut model = self.model.borrow_mut();
            *model = TabModel::new();
            // Register saved tabs lazily: only the active tab spawns a Servo
            // runner immediately; background tabs materialize on first switch.
            for (idx, saved) in session.tabs.iter().enumerate() {
                model.insert_tab_at(idx, &saved.url, &saved.title);
            }
        }

        let target_idx = session.active_index.min(session.tabs.len() - 1);
        self.select_index(target_idx);
        true
    }

    pub fn load_url_in_active_tab(&self, url: &str) {
        if let Some(id) = self.active_tab_id() {
            {
                let mut model = self.model.borrow_mut();
                model.update_url(id, url.to_string());
                model.set_loading(id, true);
            }
            if let Some(tab) = self.tab_by_id(id) {
                tab.load_url(url);
            }
            (self.callbacks.on_state_changed)();
        } else {
            self.open_url_in_new_tab(url);
        }
    }

    pub fn load_html_in_active_tab(&self, url: &str, title: &str, html: &str) {
        if let Some(id) = self.active_tab_id() {
            {
                let mut model = self.model.borrow_mut();
                model.update_url(id, url.to_string());
                model.update_title(id, title.to_string());
                model.set_loading(id, true);
            }
            if let Some(tab) = self.tab_by_id(id) {
                tab.load_html(html);
            }
            (self.callbacks.on_state_changed)();
        } else {
            self.open_html_in_new_tab(url, title, html);
        }
    }

    pub fn select_index(&self, index: usize) {
        let id = self.model.borrow_mut().select_index(index);
        if let Some(id) = id {
            self.activate_tab_id(id);
        }
    }

    pub fn next_tab(&self) {
        let id = self.model.borrow_mut().next_tab();
        if let Some(id) = id {
            self.activate_tab_id(id);
        }
    }

    pub fn prev_tab(&self) {
        let id = self.model.borrow_mut().prev_tab();
        if let Some(id) = id {
            self.activate_tab_id(id);
        }
    }

    /// Close the active tab. Returns `true` if tabs remain open, `false` if empty.
    pub fn close_active_tab(&self) -> bool {
        let removed_id = self.model.borrow_mut().close_active();
        let removed_entry = removed_id.and_then(|id| self.tabs.borrow_mut().remove(&id));
        if let Some(entry) = removed_entry {
            self.stack.remove(entry.overlay.widget());
        }

        if let Some(next_id) = self.active_tab_id() {
            self.activate_tab_id(next_id);
            true
        } else {
            (self.callbacks.on_state_changed)();
            false
        }
    }

    /// Reopen the most recently closed tab at its previous position.
    /// Returns `false` if there is no closed tab to restore.
    pub fn reopen_closed_tab(&self) -> bool {
        let Some(closed) = self.model.borrow_mut().pop_closed() else {
            return false;
        };
        let (url, web_tab) = web_tab_for_url(&closed.url);
        let id = self.insert_tab_at(closed.index, url, &closed.title, web_tab);
        self.activate_tab_id(id);
        true
    }

    pub fn active_tab(&self) -> Option<WebTab> {
        let id = self.active_tab_id()?;
        self.tab_by_id(id)
    }

    pub fn active_tab_id(&self) -> Option<u64> {
        self.model.borrow().active_tab().map(|tab| tab.id)
    }

    pub fn active_tab_info(&self) -> Option<TabInfo> {
        self.model.borrow().active_tab().cloned()
    }

    pub fn active_url(&self) -> Option<String> {
        self.model.borrow().active_tab().map(|tab| tab.url.clone())
    }

    pub fn active_web_view(&self) -> Option<WebView> {
        self.active_tab().map(|tab| tab.view().clone())
    }

    pub fn reload_active_tab(&self) {
        let Some(id) = self.active_tab_id() else {
            return;
        };
        self.model.borrow_mut().set_loading(id, true);
        if let Some(tab) = self.tab_by_id(id) {
            tab.view().reload();
        }
        (self.callbacks.on_state_changed)();
    }

    pub fn focus_active(&self) {
        if let Some(view) = self.active_web_view() {
            view.grab_focus();
        }
    }

    pub fn snapshot(&self) -> (Vec<TabInfo>, usize, f64) {
        let model = self.model.borrow();
        let tabs = model.tabs().to_vec();
        let active = model.active_index();
        let zoom = model
            .active_tab()
            .and_then(|tab| self.tab_by_id(tab.id))
            .map(|tab| tab.view().zoom_level())
            .unwrap_or(1.0);
        (tabs, active, zoom)
    }

    fn tab_by_id(&self, id: u64) -> Option<WebTab> {
        self.tabs.borrow().get(&id).map(|e| e.tab.clone())
    }

    fn activate_tab_id(&self, id: u64) {
        self.ensure_materialized(id);
        self.stack.set_visible_child_name(&tab_stack_name(id));
        if let Some(tab) = self.tab_by_id(id) {
            tab.view().grab_focus();
            tab.view().queue_resize();
            tab.view().queue_draw();
        }
        (self.callbacks.on_state_changed)();
    }

    fn ensure_materialized(&self, id: u64) {
        if self.tabs.borrow().contains_key(&id) {
            return;
        }
        let Some(url) = self.model.borrow().tabs().iter().find(|t| t.id == id).map(|t| t.url.clone()) else {
            return;
        };
        let (resolved, web_tab) = web_tab_for_url(&url);
        if resolved != url {
            self.model.borrow_mut().update_url(id, resolved.to_string());
        }
        self.attach_web_tab(id, web_tab);
    }

    fn insert_tab(&self, url: &str, title: &str, web_tab: WebTab) -> u64 {
        let end = self.model.borrow().len();
        self.insert_tab_at(end, url, title, web_tab)
    }

    fn insert_tab_at(&self, index: usize, url: &str, title: &str, web_tab: WebTab) -> u64 {
        let id = self.model.borrow_mut().insert_tab_at(index, url, title);
        self.attach_web_tab(id, web_tab);
        id
    }

    fn attach_web_tab(&self, id: u64, web_tab: WebTab) {
        self.model.borrow_mut().set_loading(id, true);
        let overlay = PageOverlay::new(&web_tab);
        wire_tab_signals(id, &web_tab, overlay.downgrade(), &self.model, &self.callbacks);
        self.stack.add_named(overlay.widget(), Some(&tab_stack_name(id)));
        self.tabs.borrow_mut().insert(id, TabEntry { tab: web_tab, overlay });
    }
}

fn tab_stack_name(id: u64) -> String {
    format!("tab-{id}")
}

fn web_tab_for_url(url: &str) -> (&str, WebTab) {
    if url == BOOKMARKS_PAGE_URL {
        let list = BookmarkStore::default_store().list().unwrap_or_default();
        (BOOKMARKS_PAGE_URL, WebTab::new_html(&render_bookmarks_html(&list)))
    } else if url == HISTORY_PAGE_URL {
        let list = HistoryStore::default_store().recent(200).unwrap_or_default();
        (HISTORY_PAGE_URL, WebTab::new_html(&render_history_html(&list, None)))
    } else if url == "about:help" || url.trim().is_empty() {
        ("about:help", WebTab::new_html(start_page::HELP_PAGE_HTML))
    } else {
        (url, WebTab::new_url(url))
    }
}
