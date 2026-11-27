use nservo::core::{LoadCoordinator, LoadTarget};

#[test]
fn normal_startup_waits_for_about_blank_handshake_then_loads_target() {
    let mut coord = LoadCoordinator::new(LoadTarget::Url("https://example.com".to_string()));
    assert!(!coord.is_ready());
    assert!(!coord.accept_uri_change("about:blank"));

    // Initial about:blank load finishes -> dispatch queued URL.
    assert_eq!(
        coord.on_load_finished(Some("about:blank")),
        Some(LoadTarget::Url("https://example.com".to_string()))
    );
    assert!(coord.is_ready());

    // Real page loads and finishes cleanly.
    assert!(coord.accept_uri_change("https://example.com/"));
    assert_eq!(coord.on_load_finished(Some("https://example.com/")), None);
}

#[test]
fn early_fallback_retries_when_servo_finishes_on_about_blank() {
    let mut coord = LoadCoordinator::new(LoadTarget::Url("https://rust-lang.org".to_string()));

    // Fallback fires before Servo's browsing context finishes initializing.
    assert_eq!(
        coord.on_fallback_timeout(),
        Some(LoadTarget::Url("https://rust-lang.org".to_string()))
    );
    assert!(!coord.is_ready());

    // Servo's delayed initial about:blank URI must NOT overwrite the tab URL.
    assert!(!coord.accept_uri_change("about:blank"));

    // Servo's delayed initial about:blank LoadFinished triggers automatic retry!
    assert_eq!(
        coord.on_load_finished(Some("about:blank")),
        Some(LoadTarget::Url("https://rust-lang.org".to_string()))
    );
    assert!(coord.is_ready());

    // Once retried load finishes on the real URL, no further retry occurs.
    assert!(coord.accept_uri_change("https://www.rust-lang.org/"));
    assert_eq!(
        coord.on_load_finished(Some("https://www.rust-lang.org/")),
        None
    );
}

#[test]
fn builtin_html_preserves_about_help_url_and_clears_on_navigation() {
    let mut coord = LoadCoordinator::new(LoadTarget::Html("<h1>Help</h1>".to_string()));
    assert_eq!(
        coord.on_load_finished(Some("about:blank")),
        Some(LoadTarget::Html("<h1>Help</h1>".to_string()))
    );

    // servo-gtk encodes load_html as a data:text/html URL; ignore it so `go` keeps `about:help`.
    assert!(!coord.accept_uri_change("data:text/html;charset=utf-8,%3Ch1%3EHelp%3C%2Fh1%3E"));
    assert_eq!(
        coord.on_load_finished(Some("data:text/html;charset=utf-8,%3Ch1%3EHelp%3C%2Fh1%3E")),
        None
    );

    // Navigating to a real URL updates normally.
    assert_eq!(
        coord.request_load(LoadTarget::Url("https://servo.org".to_string())),
        Some(LoadTarget::Url("https://servo.org".to_string()))
    );
    assert!(coord.accept_uri_change("https://servo.org/"));
}
