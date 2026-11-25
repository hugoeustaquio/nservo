use servo_gtk::WebView;

/// Default vertical/horizontal step in CSS pixels for `j`, `k`, `h`, and `l`.
pub const SCROLL_STEP_PX: i32 = 80;

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
