use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use nservo::core::{
    AddBookmarkOutcome, Bookmark, BookmarkError, BookmarkStore, BrowserAction, KeyInput, KeyMapper,
    KeyOutcome, Mode, parse_command, resolve_config_dir, resolve_db_path,
};
use nservo::engine::bookmarks_page::render_bookmarks_html;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_bookmarks_db() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("nservo-bookmarks-db-test-{pid}-{id}"));
    let _ = fs::remove_dir_all(&dir);
    dir.join("nservo.db")
}

#[test]
fn bookmarks_db_uses_xdg_config_home() {
    assert_eq!(
        resolve_config_dir(Some("/tmp/xdg-cfg"), Some("/home/alice")),
        PathBuf::from("/tmp/xdg-cfg/nservo")
    );
    assert_eq!(
        resolve_db_path(Some("/tmp/xdg-cfg"), Some("/home/alice")),
        PathBuf::from("/tmp/xdg-cfg/nservo/nservo.db")
    );
    assert_eq!(
        resolve_db_path(None, Some("/home/alice")),
        PathBuf::from("/home/alice/.config/nservo/nservo.db")
    );
}

#[test]
fn add_deduplicate_and_remove_bookmarks_in_sqlite() {
    let db_path = temp_bookmarks_db();
    let store = BookmarkStore::new(db_path.clone());
    assert!(store.list().expect("empty list").is_empty());

    let added = store
        .add("https://servo.org", "Servo Engine")
        .expect("add first bookmark");
    assert_eq!(
        added,
        AddBookmarkOutcome::Added(Bookmark {
            url: "https://servo.org".to_string(),
            title: "Servo Engine".to_string(),
        })
    );

    let dup = store
        .add("https://servo.org", "Other Title")
        .expect("duplicate check");
    assert!(matches!(dup, AddBookmarkOutcome::AlreadyExists(_)));
    assert_eq!(store.list().expect("list").len(), 1);

    // Empty title is backfilled when bookmarked again with a real title.
    store
        .add("https://rust-lang.org", "")
        .expect("add untitled");
    let updated = store
        .add("https://rust-lang.org", "Rust")
        .expect("backfill title");
    assert_eq!(
        updated,
        AddBookmarkOutcome::AlreadyExists(Bookmark {
            url: "https://rust-lang.org".to_string(),
            title: "Rust".to_string(),
        })
    );

    let removed_idx = store.remove("1").expect("remove by index");
    assert_eq!(
        removed_idx.map(|b| b.url),
        Some("https://servo.org".to_string())
    );
    let removed_url = store
        .remove("https://rust-lang.org")
        .expect("remove by url");
    assert_eq!(removed_url.map(|b| b.title), Some("Rust".to_string()));
    assert!(store.list().expect("list after remove").is_empty());

    if let Some(dir) = db_path.parent() {
        let _ = fs::remove_dir_all(dir);
    }
}

#[test]
fn internal_and_empty_urls_are_rejected() {
    let db_path = temp_bookmarks_db();
    let store = BookmarkStore::new(db_path.clone());

    assert!(matches!(
        store.add("", "Empty"),
        Err(BookmarkError::InvalidUrl(_))
    ));
    assert!(matches!(
        store.add("about:blank", "Blank"),
        Err(BookmarkError::InvalidUrl(_))
    ));
    assert!(matches!(
        store.add("about:help", "Help"),
        Err(BookmarkError::InvalidUrl(_))
    ));
    assert!(matches!(
        store.add("about:bookmarks", "Bookmarks"),
        Err(BookmarkError::InvalidUrl(_))
    ));
    assert!(matches!(
        store.add("about:history", "History"),
        Err(BookmarkError::InvalidUrl(_))
    ));

    if let Some(dir) = db_path.parent() {
        let _ = fs::remove_dir_all(dir);
    }
}

#[test]
fn keymap_and_commands_wire_bookmarks() {
    let mut mapper = KeyMapper::new();
    let now = Instant::now();

    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('b'), now),
        KeyOutcome::Action(BrowserAction::AddBookmark)
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('B'), now),
        KeyOutcome::Action(BrowserAction::ShowBookmarks)
    );

    assert_eq!(parse_command(":bookmark"), Ok(BrowserAction::AddBookmark));
    assert_eq!(
        parse_command(":bookmarks"),
        Ok(BrowserAction::ShowBookmarks)
    );
    assert_eq!(parse_command(":bm"), Ok(BrowserAction::ShowBookmarks));
    assert_eq!(
        parse_command(":bmdel"),
        Ok(BrowserAction::RemoveBookmark(None))
    );
    assert_eq!(
        parse_command(":bookmark-del 2"),
        Ok(BrowserAction::RemoveBookmark(Some("2".to_string())))
    );
}

#[test]
fn bookmarks_html_escapes_special_characters_and_renders_empty_state() {
    let empty = render_bookmarks_html(&[]);
    assert!(empty.contains("No bookmarks saved yet"));

    let html = render_bookmarks_html(&[Bookmark {
        url: "https://example.com/?a=1&b=2".to_string(),
        title: "A <script> & \"Quotes\"".to_string(),
    }]);
    assert!(html.contains("A &lt;script&gt; &amp; &quot;Quotes&quot;"));
    assert!(html.contains("https://example.com/?a=1&amp;b=2"));
}
