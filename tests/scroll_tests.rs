use nservo::core::{ScrollMetrics, parse_scroll_metrics};
use nservo::engine::scroll::{
    SCROLL_MESSAGE_HANDLER, jump_scroll_x_script, jump_scroll_y_script, scroll_hide_css,
    scroll_init_script, smooth_scroll_by_script,
};

#[test]
fn parses_valid_scroll_metrics_and_computes_scrollability() {
    let metrics = parse_scroll_metrics(
        r#"{"y":120.5,"viewH":800,"scrollH":2400,"x":0,"viewW":1200,"scrollW":1200}"#,
    )
    .expect("valid scroll metrics");

    assert_eq!(
        metrics,
        ScrollMetrics {
            y: 120.5,
            view_h: 800.0,
            scroll_h: 2400.0,
            x: 0.0,
            view_w: 1200.0,
            scroll_w: 1200.0,
        }
    );
    assert!(metrics.is_v_scrollable());
    assert_eq!(metrics.max_y(), 1600.0);
    assert!(!metrics.is_h_scrollable());
    assert_eq!(metrics.max_x(), 0.0);
}

#[test]
fn rejects_invalid_or_negative_scroll_metrics() {
    assert_eq!(parse_scroll_metrics("not json"), None);
    assert_eq!(
        parse_scroll_metrics(r#"{"y":-5,"viewH":800,"scrollH":1000}"#),
        None
    );
}

#[test]
fn scroll_scripts_and_css_include_smooth_animation_and_handler() {
    let init = scroll_init_script();
    assert!(init.contains(SCROLL_MESSAGE_HANDLER));
    assert!(init.contains("requestAnimationFrame"));
    assert!(init.contains("__nservoScroll"));

    let css = scroll_hide_css();
    assert!(css.contains("scrollbar-width: none"));
    assert!(css.contains("scroll-behavior: smooth"));

    assert!(smooth_scroll_by_script(0.0, 80.0).contains("__nservoScroll.by(0, 80)"));
    assert!(jump_scroll_y_script(250.0).contains("__nservoScroll.jumpY(250)"));
    assert!(jump_scroll_x_script(40.0).contains("__nservoScroll.jumpX(40)"));
}
