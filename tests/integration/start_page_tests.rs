use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use nservo::core::{
    Bookmark, BookmarkStore, BrowserAction, HistoryEntry, parse_command, resolve_startpage_path,
};
use nservo::engine::new_tab_page::{
    DEFAULT_SEARCH_ENGINE_ACTION, render_default_start_page_html, resolve_start_page_html,
};

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
fn tabnew_command_opens_start_page_or_optional_url() {
    assert_eq!(
        parse_command(":tabnew"),
        Ok(BrowserAction::OpenStartPageInNewTab)
    );
    assert_eq!(
        parse_command("tabnew"),
        Ok(BrowserAction::OpenStartPageInNewTab)
    );
    assert_eq!(
        parse_command(":tabnew https://servo.org"),
        Ok(BrowserAction::OpenInNewTab("https://servo.org".to_string()))
    );
    assert_eq!(
        parse_command(":tabnew rust lang"),
        Ok(BrowserAction::OpenInNewTab(
            "https://www.google.com/search?q=rust+lang".to_string()
        ))
    );
}

#[test]
fn startpage_path_uses_xdg_config_home_then_home() {
    assert_eq!(
        resolve_startpage_path(Some("/tmp/xdg"), Some("/home/alice")),
        PathBuf::from("/tmp/xdg/nservo/startpage.html")
    );
    assert_eq!(
        resolve_startpage_path(None, Some("/home/alice")),
        PathBuf::from("/home/alice/.config/nservo/startpage.html")
    );
}

#[test]
fn default_start_page_renders_search_bar_recent_bookmarks_and_history() {
    let dir = temp_dir("startpage-bm");
    let store = BookmarkStore::new(dir.join("nservo.db"));
    store
        .add("https://first.example", "First Bookmark")
        .unwrap();
    store
        .add("https://second.example", "Second <Bookmark>")
        .unwrap();

    // `list_recent` orders most recently added bookmark first.
    let recent_bms = store.list_recent(10).unwrap();
    assert_eq!(
        recent_bms,
        vec![
            Bookmark {
                url: "https://second.example".to_string(),
                title: "Second <Bookmark>".to_string(),
            },
            Bookmark {
                url: "https://first.example".to_string(),
                title: "First Bookmark".to_string(),
            },
        ]
    );

    let history = vec![HistoryEntry {
        id: 1,
        url: "https://docs.rs/gtk4".to_string(),
        title: "GTK4 Docs & Guide".to_string(),
        visited_at: 100,
    }];

    let html = render_default_start_page_html(&recent_bms, &history);
    assert!(html.contains(&format!(r#"action="{DEFAULT_SEARCH_ENGINE_ACTION}""#)));
    assert!(html.contains(r#"name="q""#));

    let second_pos = html
        .find("Second &lt;Bookmark&gt;")
        .expect("second bookmark");
    let first_pos = html.find("First Bookmark").expect("first bookmark");
    assert!(second_pos < first_pos, "newer bookmark must appear first");

    let hist_pos = html.find("GTK4 Docs &amp; Guide").expect("history entry");
    assert!(first_pos < hist_pos, "bookmarks section precedes history");
    assert!(html.contains(r#"href="https://docs.rs/gtk4""#));
}

#[test]
fn custom_startpage_html_overrides_default_when_present() {
    let dir = temp_dir("startpage-custom");
    let custom = dir.join("startpage.html");

    // Missing file falls back to default starter page.
    let fallback = resolve_start_page_html(&custom, &[], &[]);
    assert!(fallback.contains(DEFAULT_SEARCH_ENGINE_ACTION));

    // Present file is returned verbatim.
    let custom_html = "<!doctype html><title>Custom Start</title><h1>Welcome</h1>";
    fs::write(&custom, custom_html).expect("write custom startpage.html");
    assert_eq!(resolve_start_page_html(&custom, &[], &[]), custom_html);
}
