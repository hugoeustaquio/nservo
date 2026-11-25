use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk::prelude::*;
use servo_gtk::{LoadEvent, WebView};

use crate::core::{HintEvent, Session, TabInfo, TabModel, parse_hint_message};
use crate::engine::hints_js::HINT_MESSAGE_HANDLER;
use crate::engine::start_page;
use crate::engine::web_tab::WebTab;

/// Callbacks emitted when tab metadata changes, a popup opens, or hints report an event.
#[derive(Clone)]
pub struct TabViewCallbacks {
    pub on_state_changed: Rc<dyn Fn()>,
    pub on_popup_url: Rc<dyn Fn(String)>,
    pub on_hint_event: Rc<dyn Fn(u64, HintEvent)>,
}

/// Hosts one [`WebTab`] per tab inside a [`gtk::Stack`].
#[derive(Clone)]
pub struct TabView {
    stack: gtk::Stack,
    model: Rc<RefCell<TabModel>>,
    tabs: Rc<RefCell<HashMap<u64, WebTab>>>,
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

    pub fn open_html_in_new_tab(&self, title: &str, html: &str) {
        let id = self.insert_tab("about:help", title, WebTab::new_html(html));
        self.activate_tab_id(id);
    }

    /// Replace all open tabs with the tabs stored in `session`.
    /// Returns `false` if `session` contains no tabs.
    pub fn restore_session(&self, session: &Session) -> bool {
        if session.tabs.is_empty() {
            return false;
        }

        let old_tabs: Vec<WebTab> = self.tabs.borrow_mut().drain().map(|(_, t)| t).collect();
        for tab in old_tabs {
            self.stack.remove(tab.view());
        }
        *self.model.borrow_mut() = TabModel::new();

        for saved in &session.tabs {
            let web_tab = if saved.url == "about:help" {
                WebTab::new_html(start_page::HELP_PAGE_HTML)
            } else {
                WebTab::new_url(&saved.url)
            };
            self.insert_tab(&saved.url, &saved.title, web_tab);
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

    pub fn load_html_in_active_tab(&self, title: &str, html: &str) {
        if let Some(id) = self.active_tab_id() {
            {
                let mut model = self.model.borrow_mut();
                model.update_url(id, "about:help".to_string());
                model.update_title(id, title.to_string());
                model.set_loading(id, true);
            }
            if let Some(tab) = self.tab_by_id(id) {
                tab.load_html(html);
            }
            (self.callbacks.on_state_changed)();
        } else {
            self.open_html_in_new_tab(title, html);
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
        let removed_tab = removed_id.and_then(|id| self.tabs.borrow_mut().remove(&id));
        if let Some(tab) = removed_tab {
            self.stack.remove(tab.view());
        }

        if let Some(next_id) = self.active_tab_id() {
            self.activate_tab_id(next_id);
            true
        } else {
            (self.callbacks.on_state_changed)();
            false
        }
    }

    pub fn active_tab(&self) -> Option<WebTab> {
        let id = self.active_tab_id()?;
        self.tab_by_id(id)
    }

    pub fn active_tab_id(&self) -> Option<u64> {
        self.model.borrow().active_tab().map(|tab| tab.id)
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
        self.tabs.borrow().get(&id).cloned()
    }

    fn activate_tab_id(&self, id: u64) {
        self.stack.set_visible_child_name(&tab_stack_name(id));
        if let Some(tab) = self.tab_by_id(id) {
            tab.view().grab_focus();
            tab.view().queue_resize();
            tab.view().queue_draw();
        }
        (self.callbacks.on_state_changed)();
    }

    fn insert_tab(&self, url: &str, title: &str, web_tab: WebTab) -> u64 {
        let id = self.model.borrow_mut().add_tab(url, title);
        self.model.borrow_mut().set_loading(id, true);

        self.wire_signals(id, &web_tab);
        self.stack
            .add_named(web_tab.view(), Some(&tab_stack_name(id)));
        self.tabs.borrow_mut().insert(id, web_tab);
        id
    }

    fn wire_signals(&self, id: u64, web_tab: &WebTab) {
        let view = web_tab.view();

        let model = self.model.clone();
        let on_change = self.callbacks.on_state_changed.clone();
        let tab = web_tab.clone();
        view.connect_uri_notify(move |v| {
            let Some(uri) = v.uri() else {
                return;
            };
            if !tab.is_ready() && uri == "about:blank" {
                return;
            }
            model.borrow_mut().update_url(id, uri);
            on_change();
        });

        let model = self.model.clone();
        let on_change = self.callbacks.on_state_changed.clone();
        view.connect_title_notify(move |v| {
            let title = v.title().unwrap_or_default();
            model.borrow_mut().update_title(id, title);
            on_change();
        });

        let model = self.model.clone();
        let on_change = self.callbacks.on_state_changed.clone();
        let tab = web_tab.clone();
        view.connect_load_changed(move |_, event| {
            if matches!(event, LoadEvent::Finished) {
                if tab.mark_ready_and_flush() {
                    return;
                }
                tab.nudge_repaint();
            }
            let loading = matches!(event, LoadEvent::Started);
            model.borrow_mut().set_loading(id, loading);
            on_change();
        });

        let on_change = self.callbacks.on_state_changed.clone();
        view.connect_zoom_level_notify(move |_| on_change());

        let on_popup = self.callbacks.on_popup_url.clone();
        view.connect_create_web_view(move |_, url| on_popup(url.to_string()));

        let on_hint = self.callbacks.on_hint_event.clone();
        web_tab
            .user_content()
            .connect_script_message_received(move |_, name, body| {
                if name == HINT_MESSAGE_HANDLER
                    && let Some(event) = parse_hint_message(body)
                {
                    on_hint(id, event);
                }
            });
    }
}

fn tab_stack_name(id: u64) -> String {
    format!("tab-{id}")
}
