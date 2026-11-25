use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use nservo::core::{
    DEFAULT_SESSION_NAME, Session, SessionError, SessionStore, TabInfo, validate_session_name,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_session_dir() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("nservo-session-test-{pid}-{id}"));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn save_and_restore_default_and_named_sessions() {
    let dir = temp_session_dir();
    let store = SessionStore::new(dir.clone());

    let tabs = vec![
        TabInfo {
            id: 1,
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            is_loading: false,
        },
        TabInfo {
            id: 2,
            url: "https://rust-lang.org".to_string(),
            title: "Rust".to_string(),
            is_loading: true,
        },
    ];

    let default_session = Session::from_tabs(DEFAULT_SESSION_NAME, &tabs, 1);
    store.save(&default_session).expect("save default session");

    let named_session = Session::from_tabs("work-1", &tabs[..1], 0);
    store.save(&named_session).expect("save named session");

    let loaded_default = store
        .load_optional(DEFAULT_SESSION_NAME)
        .expect("load optional default")
        .expect("default session should exist");
    assert_eq!(loaded_default, default_session);
    assert_eq!(loaded_default.active_index, 1);

    let loaded_named = store.load("work-1").expect("load named session");
    assert_eq!(loaded_named, named_session);
    assert_eq!(loaded_named.tabs.len(), 1);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn missing_session_returns_not_found_and_optional_none() {
    let dir = temp_session_dir();
    let store = SessionStore::new(dir.clone());

    assert!(matches!(
        store.load("missing"),
        Err(SessionError::NotFound(name)) if name == "missing"
    ));
    assert_eq!(
        store.load_optional("default").expect("optional missing"),
        None
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn invalid_session_names_are_rejected() {
    assert!(validate_session_name("default").is_ok());
    assert!(validate_session_name("my_work-2").is_ok());
    assert!(validate_session_name("").is_err());
    assert!(validate_session_name("../evil").is_err());
    assert!(validate_session_name("a/b").is_err());
    assert!(validate_session_name("two words").is_err());
}
