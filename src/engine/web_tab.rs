use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use servo_gtk::{UserContentManager, WebView};

use crate::core::encode_utf16_units;
use crate::engine::hints_js::{
    HINT_MESSAGE_HANDLER, activate_hints_script, clear_hints_script, filter_hints_script,
};
use crate::engine::text_input_js::insert_utf16_script;

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
