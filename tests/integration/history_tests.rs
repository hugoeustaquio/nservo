use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use nservo::core::{
    BookmarkStore, BrowserAction, HistoryEntry, HistoryStore, KeyInput, KeyMapper, KeyOutcome,
    Mode, Session, SessionStore, TabInfo, is_recordable_history_url, parse_command,
};
use nservo::engine::history_page::render_history_html;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_db_path() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("nservo-history-db-test-{pid}-{id}"));
    let _ = fs::remove_dir_all(&dir);
    dir.join("nservo.db")
}

#[test]
fn records_updates_searches_and_clears_history_in_sqlite() {
    let db_path = temp_db_path();
    let store = HistoryStore::new(db_path.clone());

    assert!(!store.record_visit("", "").expect("empty skipped"));
    assert!(!store.record_visit("about:help", "Help").expect("about skipped"));
    assert!(!store.record_visit("data:text/html,hi", "").expect("data skipped"));
    assert!(is_recordable_history_url("https://servo.org"));

    assert!(store.record_visit("https://servo.org", "").expect("visit 1"));
    // Consecutive visit to same URL coalesces and fills in title.
    assert!(
        store
            .record_visit("https://servo.org", "Servo Web Engine")
            .expect("visit 1 coalesced")
    );
    assert!(
        store
            .record_visit("https://rust-lang.org", "")
            .expect("visit 2")
    );
    assert!(
        store
            .update_title("https://rust-lang.org", "Rust Programming Language")
            .expect("update title")
    );

    let recent = store.recent(10).expect("recent history");
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].url, "https://rust-lang.org");
    assert_eq!(recent[0].title, "Rust Programming Language");
    assert_eq!(recent[1].url, "https://servo.org");
    assert_eq!(recent[1].title, "Servo Web Engine");

    let filtered = store.search("servo", 10).expect("search history");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].url, "https://servo.org");

    // Wildcard characters in search queries are escaped literally.
    assert!(store.search("%", 10).expect("escaped wildcard").is_empty());

    assert_eq!(store.clear().expect("clear history"), 2);
    assert!(store.recent(10).expect("empty after clear").is_empty());

    if let Some(dir) = db_path.parent() {
        let _ = fs::remove_dir_all(dir);
    }
}

#[test]
fn sessions_bookmarks_and_history_share_one_sqlite_database() {
    let db_path = temp_db_path();
    let sessions = SessionStore::new(db_path.clone());
    let bookmarks = BookmarkStore::new(db_path.clone());
    let history = HistoryStore::new(db_path.clone());

    let tabs = vec![TabInfo {
        id: 1,
        url: "https://servo.org".to_string(),
        title: "Servo".to_string(),
        is_loading: false,
    }];
    sessions
        .save(&Session::from_tabs("default", &tabs, 0))
        .expect("save session");
    bookmarks
        .add("https://servo.org", "Servo")
        .expect("add bookmark");
    history
        .record_visit("https://servo.org", "Servo")
        .expect("record history");

    assert_eq!(sessions.load("default").expect("load session").tabs.len(), 1);
    assert_eq!(bookmarks.list().expect("list bookmarks").len(), 1);
    assert_eq!(history.recent(10).expect("recent history").len(), 1);

    if let Some(dir) = db_path.parent() {
        let _ = fs::remove_dir_all(dir);
    }
}

#[test]
fn keymap_commands_and_html_support_history() {
    let mut mapper = KeyMapper::new();
    let t0 = Instant::now();
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('g'), t0),
        KeyOutcome::Pending("g".to_string())
    );
    assert_eq!(
        mapper.handle_key(
            Mode::Normal,
            KeyInput::Char('h'),
            t0 + Duration::from_millis(120)
        ),
        KeyOutcome::Action(BrowserAction::ShowHistory(None))
    );

    assert_eq!(parse_command(":history"), Ok(BrowserAction::ShowHistory(None)));
    assert_eq!(
        parse_command(":hist rust"),
        Ok(BrowserAction::ShowHistory(Some("rust".to_string())))
    );
    assert_eq!(
        parse_command(":history-clear"),
        Ok(BrowserAction::ClearHistory)
    );
    assert_eq!(parse_command(":histclear"), Ok(BrowserAction::ClearHistory));

    let empty = render_history_html(&[], None);
    assert!(empty.contains("No matching browsing history entries"));

    let html = render_history_html(
        &[HistoryEntry {
            id: 1,
            url: "https://example.com/?a=1&b=2".to_string(),
            title: "<b>Example</b>".to_string(),
            visited_at: 1_700_000_000,
        }],
        Some("example"),
    );
    assert!(html.contains("&lt;b&gt;Example&lt;/b&gt;"));
    assert!(html.contains("https://example.com/?a=1&amp;b=2"));
    assert!(html.contains("Filtered by <code>example</code>"));
}
