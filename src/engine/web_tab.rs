use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use servo_gtk::{UserContentManager, UserScript, UserStyleSheet, WebView};

use crate::core::{
    LineEdge, LoadCoordinator, LoadTarget, WordDirection, default_adblock_dir, encode_utf16_units,
};
use crate::engine::clipboard::{
    CLIPBOARD_MESSAGE_HANDLER, copy_selection_script, cut_selection_script,
    parse_clipboard_message, paste_into_tab, write_system_clipboard,
};
use crate::engine::config_page::{CONFIG_MESSAGE_HANDLER, config_feedback_script};
use crate::engine::hints_js::{
    HINT_MESSAGE_HANDLER, activate_hints_script, clear_hints_script, filter_hints_script,
};
use crate::engine::inspector_js::{
    INSPECTOR_MESSAGE_HANDLER, clear_highlight_script, eval_in_page_script, inspect_element_script,
    inspector_init_script, snapshot_dom_script,
};
use crate::engine::inspector_picker_js::{
    start_element_picker_script, stop_element_picker_script,
};
use crate::engine::scroll::{
    SCROLL_MESSAGE_HANDLER, jump_scroll_x_script, jump_scroll_y_script, scroll_hide_css,
    scroll_init_script, smooth_scroll_by_script,
};
use crate::engine::search_js::{
    SEARCH_MESSAGE_HANDLER, clear_search_script, search_query_script, search_step_script,
};
use crate::engine::text_input_js::{insert_utf16_script, move_line_edge_script, move_word_script};

const READY_FALLBACK_TIMEOUT: Duration = Duration::from_millis(5000);

/// Wraps a `servo_gtk::WebView` with readiness-aware load queuing.
///
/// Newly spawned `servo-gtk` runners initialize their browsing context
/// asynchronously and finish with an initial `about:blank` load completion.
/// Queuing loads until that handshake completes prevents Servo from dropping
/// navigation requests with `LoadUrl for unknown browsing context`.
#[derive(Clone)]
pub struct WebTab {
    view: WebView,
    user_content: UserContentManager,
    state: Rc<RefCell<LoadCoordinator>>,
}

/// Weak handle to a [`WebTab`] used inside GTK signal handlers and timers so
/// closing a tab drops its [`WebView`] (triggering `dispose()` and shutting
/// down the Servo runner subprocess) without a GObject reference cycle.
#[derive(Clone)]
pub struct WeakWebTab {
    view: glib::WeakRef<WebView>,
    user_content: glib::WeakRef<UserContentManager>,
    state: Weak<RefCell<LoadCoordinator>>,
}

impl WeakWebTab {
    pub fn upgrade(&self) -> Option<WebTab> {
        Some(WebTab {
            view: self.view.upgrade()?,
            user_content: self.user_content.upgrade()?,
            state: self.state.upgrade()?,
        })
    }
}

impl WebTab {
    pub fn new_url(url: &str) -> Self {
        Self::new_with_initial(LoadTarget::Url(url.to_string()))
    }

    pub fn new_html(html: &str) -> Self {
        Self::new_with_initial(LoadTarget::Html(html.to_string()))
    }

    fn new_with_initial(initial: LoadTarget) -> Self {
        let user_content = UserContentManager::new();
        user_content.register_script_message_handler(HINT_MESSAGE_HANDLER);
        user_content.register_script_message_handler(CLIPBOARD_MESSAGE_HANDLER);
        user_content.register_script_message_handler(CONFIG_MESSAGE_HANDLER);
        user_content.register_script_message_handler(INSPECTOR_MESSAGE_HANDLER);
        user_content.register_script_message_handler(SCROLL_MESSAGE_HANDLER);
        user_content.register_script_message_handler(SEARCH_MESSAGE_HANDLER);
        user_content.add_style_sheet(&UserStyleSheet::new(scroll_hide_css()));
        user_content.add_script(&UserScript::new(inspector_init_script()));
        user_content.add_script(&UserScript::new(scroll_init_script()));
        user_content.connect_script_message_received(|_, name, body| {
            if name == CLIPBOARD_MESSAGE_HANDLER
                && let Some(text) = parse_clipboard_message(body)
            {
                write_system_clipboard(&text);
            }
        });

        let view = WebView::with_user_content_manager(&user_content);
        view.set_hexpand(true);
        view.set_vexpand(true);
        view.set_adblock_dir(&default_adblock_dir().to_string_lossy());

        let tab = Self { view, user_content, state: Rc::new(RefCell::new(LoadCoordinator::new(initial))) };
        tab.schedule_ready_fallback();
        tab
    }

    pub fn reload_adblock(&self, dir: &str) { self.view.set_adblock_dir(dir); }
    pub fn devtools_endpoint(&self) -> Option<(u16, String)> { self.view.devtools_endpoint() }
    pub fn notify_config_result(&self, ok: bool, msg: &str) {
        self.view.evaluate_javascript(&config_feedback_script(ok, msg));
    }
    pub fn view(&self) -> &WebView { &self.view }

    pub fn downgrade(&self) -> WeakWebTab {
        WeakWebTab {
            view: self.view.downgrade(),
            user_content: self.user_content.downgrade(),
            state: Rc::downgrade(&self.state),
        }
    }

    pub fn user_content(&self) -> &UserContentManager {
        &self.user_content
    }

    pub fn start_hints(&self, new_tab: bool) {
        self.view.evaluate_javascript(&activate_hints_script(new_tab));
    }

    pub fn filter_hints(&self, typed: &str) {
        self.view.evaluate_javascript(&filter_hints_script(typed));
    }

    pub fn clear_hints(&self) {
        self.view.evaluate_javascript(clear_hints_script());
    }

    /// Encode `text` as UTF-16 code units and insert it into the focused input.
    pub fn insert_utf16_text(&self, text: &str) {
        let units = encode_utf16_units(text);
        if !units.is_empty() {
            self.view.evaluate_javascript(&insert_utf16_script(&units));
        }
    }

    /// Move or extend selection by one word in the focused page input.
    pub fn move_word(&self, direction: WordDirection, select: bool) {
        self.view
            .evaluate_javascript(&move_word_script(direction, select));
    }

    /// Move caret to the start (`Ctrl+A`) or end (`Ctrl+E`) of the current line.
    pub fn move_line_edge(&self, edge: LineEdge) {
        self.view.evaluate_javascript(&move_line_edge_script(edge));
    }

    /// Copy the page or input's currently selected text to the system clipboard.
    pub fn copy_selection(&self) {
        self.view.evaluate_javascript(copy_selection_script());
    }

    /// Cut selected text from the focused page input to the system clipboard.
    pub fn cut_selection(&self) {
        self.view.evaluate_javascript(cut_selection_script());
    }

    /// Paste text from the system clipboard into the focused page input.
    pub fn paste_from_clipboard(&self) {
        paste_into_tab(self.clone());
    }

    /// Evaluate a JavaScript expression in the page and report its result to the inspector.
    pub fn eval_inspector_js(&self, expression: &str) {
        self.view
            .evaluate_javascript(&eval_in_page_script(expression));
    }

    /// Request a fresh DOM tree snapshot for the built-in inspector.
    pub fn request_dom_snapshot(&self) {
        self.view.evaluate_javascript(snapshot_dom_script());
    }

    /// Inspect and highlight the `index`-th DOM node from the last snapshot.
    pub fn inspect_dom_node(&self, index: usize) {
        self.view
            .evaluate_javascript(&inspect_element_script(index));
    }

    /// Remove any DOM inspector highlight overlay from the page.
    pub fn clear_inspector_highlight(&self) {
        self.view.evaluate_javascript(clear_highlight_script());
    }

    /// Start interactive element-picker ("Find") mode in the page.
    pub fn start_element_picker(&self) {
        self.view.evaluate_javascript(start_element_picker_script());
    }

    /// Stop interactive element-picker ("Find") mode in the page.
    pub fn stop_element_picker(&self) {
        self.view.evaluate_javascript(stop_element_picker_script());
    }

    pub fn smooth_scroll_by(&self, dx: f64, dy: f64) {
        self.view
            .evaluate_javascript(&smooth_scroll_by_script(dx, dy));
    }

    pub fn jump_scroll_y(&self, y: f64) {
        self.view.evaluate_javascript(&jump_scroll_y_script(y));
    }

    pub fn jump_scroll_x(&self, x: f64) {
        self.view.evaluate_javascript(&jump_scroll_x_script(x));
    }

    pub fn search_page(&self, query: &str) {
        self.view.evaluate_javascript(&search_query_script(query));
    }

    pub fn search_step(&self, delta: i32, fallback_query: &str) {
        self.view
            .evaluate_javascript(&search_step_script(delta, fallback_query));
    }

    pub fn clear_search(&self) {
        self.view.evaluate_javascript(clear_search_script());
    }

    pub fn is_ready(&self) -> bool {
        self.state.borrow().is_ready()
    }

    pub fn accept_uri_change(&self, uri: &str) -> bool {
        self.state.borrow_mut().accept_uri_change(uri)
    }

    pub fn load_url(&self, url: &str) {
        self.dispatch_or_queue(LoadTarget::Url(url.to_string()));
    }

    pub fn load_html(&self, html: &str) {
        self.dispatch_or_queue(LoadTarget::Html(html.to_string()));
    }

    /// Handle Servo's `LoadEvent::Finished`, dispatching any queued initial or
    /// dropped fallback load. Returns `true` if a load was just dispatched.
    pub fn on_load_finished(&self, current_uri: Option<&str>) -> bool {
        let to_load = self.state.borrow_mut().on_load_finished(current_uri);
        if let Some(load) = to_load {
            apply_load(&self.view, &load);
            true
        } else {
            false
        }
    }

    fn dispatch_or_queue(&self, load: LoadTarget) {
        let to_load = self.state.borrow_mut().request_load(load);
        if let Some(load) = to_load {
            apply_load(&self.view, &load);
        }
    }

    fn schedule_ready_fallback(&self) {
        let weak = self.downgrade();
        glib::timeout_add_local_once(READY_FALLBACK_TIMEOUT, move || {
            if let Some(tab) = weak.upgrade() {
                let to_load = tab.state.borrow_mut().on_fallback_timeout();
                if let Some(load) = to_load {
                    apply_load(&tab.view, &load);
                }
            }
        });
    }
}

fn apply_load(view: &WebView, load: &LoadTarget) {
    match load {
        LoadTarget::Url(url) => view.load_url(url),
        LoadTarget::Html(html) => view.load_html(html, None),
    }
}
