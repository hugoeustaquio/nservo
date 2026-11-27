use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use nservo::core::resolve_style_path;
use nservo::ui::style::{DEFAULT_THEME_CSS, resolve_theme_css};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("nservo-{tag}-{pid}-{id}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn style_path_uses_xdg_config_home_then_home() {
    assert_eq!(
        resolve_style_path(Some("/tmp/xdg"), Some("/home/alice")),
        PathBuf::from("/tmp/xdg/nservo/style.css")
    );
    assert_eq!(
        resolve_style_path(None, Some("/home/alice")),
        PathBuf::from("/home/alice/.config/nservo/style.css")
    );
}

#[test]
fn custom_style_css_overrides_default_asset_when_present() {
    let dir = temp_dir("style-custom");
    let custom = dir.join("style.css");

    // Missing file falls back to the built-in assets/style.css theme.
    let fallback = resolve_theme_css(&custom);
    assert_eq!(fallback, DEFAULT_THEME_CSS);
    assert!(fallback.contains("window.nservo-window"));
    assert!(fallback.contains(".tab-bar"));
    assert!(fallback.contains(".status-bar"));

    // Present file overrides the built-in stylesheet verbatim.
    let custom_css = "window.nservo-window { background-color: #ffffff; color: #000000; }\n";
    fs::write(&custom, custom_css).expect("write custom style.css");
    assert_eq!(resolve_theme_css(&custom), custom_css);

    let _ = fs::remove_dir_all(&dir);
}
