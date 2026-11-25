use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use servo_gtk::{UserContentManager, WebView};

use crate::core::{LineEdge, WordDirection, encode_utf16_units};
use crate::engine::clipboard::{
    CLIPBOARD_MESSAGE_HANDLER, copy_selection_script, cut_selection_script,
    parse_clipboard_message, paste_into_tab, write_system_clipboard,
};
use crate::engine::hints_js::{
    HINT_MESSAGE_HANDLER, activate_hints_script, clear_hints_script, filter_hints_script,
};
use crate::engine::text_input_js::{insert_utf16_script, move_line_edge_script, move_word_script};

const READY_FALLBACK_TIMEOUT: Duration = Duration::from_millis(1500);

#[derive(Clone)]
enum PendingLoad {
    Url(String),
    Html(String),
}

struct LoadState {
    ready: bool,
    pending: Option<PendingLoad>,
}

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
    state: Rc<RefCell<LoadState>>,
}

impl WebTab {
    pub fn new_url(url: &str) -> Self {
        Self::new_with_initial(PendingLoad::Url(url.to_string()))
    }

    pub fn new_html(html: &str) -> Self {
        Self::new_with_initial(PendingLoad::Html(html.to_string()))
    }

    fn new_with_initial(initial: PendingLoad) -> Self {
        let user_content = UserContentManager::new();
        user_content.register_script_message_handler(HINT_MESSAGE_HANDLER);
        user_content.register_script_message_handler(CLIPBOARD_MESSAGE_HANDLER);
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

        let tab = Self {
            view,
            user_content,
            state: Rc::new(RefCell::new(LoadState {
                ready: false,
                pending: Some(initial),
            })),
        };
        tab.schedule_ready_fallback();
        tab
    }

    pub fn view(&self) -> &WebView {
        &self.view
    }

    pub fn user_content(&self) -> &UserContentManager {
        &self.user_content
    }

    pub fn start_hints(&self) {
        self.view.evaluate_javascript(activate_hints_script());
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

    pub fn is_ready(&self) -> bool {
        self.state.borrow().ready
    }

    pub fn load_url(&self, url: &str) {
        self.dispatch_or_queue(PendingLoad::Url(url.to_string()));
    }

    pub fn load_html(&self, html: &str) {
        self.dispatch_or_queue(PendingLoad::Html(html.to_string()));
    }

    /// Trigger a lightweight compositor frame after a page finishes loading so
    /// static documents flush through `servo-gtk`'s rendering pipeline without
    /// waiting for user input.
    pub fn nudge_repaint(&self) {
        self.view.evaluate_javascript(
            "requestAnimationFrame(function() {\
                var el = document.documentElement;\
                if (!el) return;\
                var prev = el.style.opacity;\
                el.style.opacity = '0.999';\
                requestAnimationFrame(function() { el.style.opacity = prev; });\
            });",
        );
    }

    /// Mark the browsing context ready and flush any queued load.
    ///
    /// Returns `true` if a queued load was just dispatched.
    pub fn mark_ready_and_flush(&self) -> bool {
        let pending = {
            let mut state = self.state.borrow_mut();
            if state.ready {
                return false;
            }
            state.ready = true;
            state.pending.take()
        };

        if let Some(load) = pending {
            apply_load(&self.view, &load);
            true
        } else {
            false
        }
    }

    fn dispatch_or_queue(&self, load: PendingLoad) {
        let is_ready = {
            let mut state = self.state.borrow_mut();
            if state.ready {
                true
            } else {
                state.pending = Some(load.clone());
                false
            }
        };

        if is_ready {
            apply_load(&self.view, &load);
        }
    }

    fn schedule_ready_fallback(&self) {
        let tab = self.clone();
        glib::timeout_add_local_once(READY_FALLBACK_TIMEOUT, move || {
            tab.mark_ready_and_flush();
        });
    }
}

fn apply_load(view: &WebView, load: &PendingLoad) {
    match load {
        PendingLoad::Url(url) => view.load_url(url),
        PendingLoad::Html(html) => view.load_html(html, None),
    }
}
