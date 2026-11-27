use nservo::core::{BrowserAction, ScrollMetrics, parse_scroll_metrics};
use nservo::engine::scroll::{
    SCROLL_MESSAGE_HANDLER, SCROLL_STEP_PX, jump_scroll_x_script, jump_scroll_y_script,
    scroll_action_script, scroll_hide_css, scroll_init_script, scroll_step_distance,
    smooth_scroll_by_script,
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
    assert!(init.contains("performance.now"));
    assert!(init.contains("behavior: 'instant'"));
    assert!(init.contains("__nservoScroll"));

    let css = scroll_hide_css();
    assert!(css.contains("scrollbar-width: none"));
    assert!(css.contains("scroll-behavior: smooth"));

    assert!(smooth_scroll_by_script(0.0, 120.0).contains("__nservoScroll.by(0, 120)"));
    assert!(jump_scroll_y_script(250.0).contains("__nservoScroll.jumpY(250)"));
    assert!(jump_scroll_x_script(40.0).contains("__nservoScroll.jumpX(40)"));
}

#[test]
fn counted_scroll_actions_multiply_step_distance() {
    assert_eq!(scroll_step_distance(0), f64::from(SCROLL_STEP_PX));
    assert_eq!(scroll_step_distance(1), f64::from(SCROLL_STEP_PX));
    assert_eq!(scroll_step_distance(12), f64::from(SCROLL_STEP_PX) * 12.0);

    let down12 = scroll_action_script(&BrowserAction::ScrollDown(12)).expect("down script");
    assert!(down12.contains("__nservoScroll.by(0, 1440)"));

    let up5 = scroll_action_script(&BrowserAction::ScrollUp(5)).expect("up script");
    assert!(up5.contains("__nservoScroll.by(0, -600)"));

    let left3 = scroll_action_script(&BrowserAction::ScrollLeft(3)).expect("left script");
    assert!(left3.contains("__nservoScroll.by(-360, 0)"));

    let right4 = scroll_action_script(&BrowserAction::ScrollRight(4)).expect("right script");
    assert!(right4.contains("__nservoScroll.by(480, 0)"));

    let top = scroll_action_script(&BrowserAction::ScrollTop).expect("top script");
    assert!(top.contains("__nservoScroll.toTop()"));

    let bottom = scroll_action_script(&BrowserAction::ScrollBottom).expect("bottom script");
    assert!(bottom.contains("__nservoScroll.toBottom()"));

    assert!(scroll_action_script(&BrowserAction::Reload).is_none());
}
