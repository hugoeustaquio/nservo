use servo_gtk::WebView;

use crate::core::BrowserAction;

/// Default vertical/horizontal step in CSS pixels for `j`, `k`, `h`, and `l`.
pub const SCROLL_STEP_PX: i32 = 80;

/// Apply a scroll-related [`BrowserAction`] to `web_view`, returning `true` if handled.
pub fn try_apply(web_view: &WebView, action: &BrowserAction) -> bool {
    match action {
        BrowserAction::ScrollDown => scroll_down(web_view),
        BrowserAction::ScrollUp => scroll_up(web_view),
        BrowserAction::ScrollLeft => scroll_left(web_view),
        BrowserAction::ScrollRight => scroll_right(web_view),
        BrowserAction::ScrollTop => scroll_top(web_view),
        BrowserAction::ScrollBottom => scroll_bottom(web_view),
        _ => return false,
    }
    true
}

pub fn scroll_down(web_view: &WebView) {
    web_view.evaluate_javascript(&format!("window.scrollBy(0, {SCROLL_STEP_PX});"));
}

pub fn scroll_up(web_view: &WebView) {
    web_view.evaluate_javascript(&format!("window.scrollBy(0, -{SCROLL_STEP_PX});"));
}

pub fn scroll_left(web_view: &WebView) {
    web_view.evaluate_javascript(&format!("window.scrollBy(-{SCROLL_STEP_PX}, 0);"));
}

pub fn scroll_right(web_view: &WebView) {
    web_view.evaluate_javascript(&format!("window.scrollBy({SCROLL_STEP_PX}, 0);"));
}

pub fn scroll_top(web_view: &WebView) {
    web_view.evaluate_javascript("window.scrollTo(0, 0);");
}

pub fn scroll_bottom(web_view: &WebView) {
    web_view.evaluate_javascript(
        "window.scrollTo(0, Math.max(\
            document.body ? document.body.scrollHeight : 0, \
            document.documentElement ? document.documentElement.scrollHeight : 0\
        ));",
    );
}
