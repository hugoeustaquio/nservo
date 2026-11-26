use std::cell::RefCell;
use std::rc::Rc;

use servo_gtk::LoadEvent;

use crate::core::{TabModel, parse_hint_message, parse_inspector_message, parse_scroll_metrics};
use crate::engine::hints_js::HINT_MESSAGE_HANDLER;
use crate::engine::inspector_js::INSPECTOR_MESSAGE_HANDLER;
use crate::engine::scroll::SCROLL_MESSAGE_HANDLER;
use crate::engine::web_tab::WebTab;
use crate::ui::page_overlay::WeakPageOverlay;
use crate::ui::tab_view::TabViewCallbacks;

/// Connect Servo `WebView` and `UserContentManager` signals for a tab.
pub(super) fn wire_tab_signals(
    id: u64,
    web_tab: &WebTab,
    overlay_weak: WeakPageOverlay,
    model: &Rc<RefCell<TabModel>>,
    callbacks: &TabViewCallbacks,
) {
    let view = web_tab.view();

    let (m, on_change, on_nav, tab_weak) = (
        model.clone(),
        callbacks.on_state_changed.clone(),
        callbacks.on_navigation.clone(),
        web_tab.downgrade(),
    );
    view.connect_uri_notify(move |v| {
        let Some(tab) = tab_weak.upgrade() else {
            return;
        };
        let Some(uri) = v.uri() else { return };
        if !tab.accept_uri_change(&uri) {
            return;
        }
        let changed = m.borrow_mut().update_url(id, uri);
        on_change();
        if tab.is_ready() && changed {
            on_nav(id);
        }
    });

    let (m, on_change) = (model.clone(), callbacks.on_state_changed.clone());
    view.connect_title_notify(move |v| {
        m.borrow_mut()
            .update_title(id, v.title().unwrap_or_default());
        on_change();
    });

    let (m, on_change, on_nav, tab_weak) = (
        model.clone(),
        callbacks.on_state_changed.clone(),
        callbacks.on_navigation.clone(),
        web_tab.downgrade(),
    );
    view.connect_load_changed(move |v, event| {
        let Some(tab) = tab_weak.upgrade() else {
            return;
        };
        if matches!(event, LoadEvent::Finished) {
            if tab.on_load_finished(v.uri().as_deref()) {
                return;
            }
            tab.nudge_repaint();
        }
        let loading = matches!(event, LoadEvent::Started);
        m.borrow_mut().set_loading(id, loading);
        on_change();
        if loading && tab.is_ready() {
            on_nav(id);
        }
    });

    let on_change = callbacks.on_state_changed.clone();
    view.connect_zoom_level_notify(move |_| on_change());

    let on_popup = callbacks.on_popup_url.clone();
    view.connect_create_web_view(move |_, url| on_popup(url.to_string()));

    let on_hint = callbacks.on_hint_event.clone();
    let on_inspector = callbacks.on_inspector_event.clone();
    web_tab
        .user_content()
        .connect_script_message_received(move |_, name, body| {
            if name == HINT_MESSAGE_HANDLER
                && let Some(event) = parse_hint_message(body)
            {
                on_hint(id, event);
            } else if name == INSPECTOR_MESSAGE_HANDLER
                && let Some(event) = parse_inspector_message(body)
            {
                on_inspector(id, event);
            } else if name == SCROLL_MESSAGE_HANDLER
                && let Some(metrics) = parse_scroll_metrics(body)
            {
                overlay_weak.update_metrics(&metrics);
            }
        });
}
