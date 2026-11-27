use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use nservo::core::{
    DEFAULT_SESSION_NAME, Session, SessionError, SessionStore, TabInfo, resolve_config_dir,
    resolve_db_path, validate_session_name,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_db_path() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("nservo-session-db-test-{pid}-{id}"));
    let _ = fs::remove_dir_all(&dir);
    dir.join("nservo.db")
}

#[test]
fn save_overwrite_and_restore_default_and_named_sessions() {
    let db_path = temp_db_path();
    let store = SessionStore::new(db_path.clone());

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

    // Overwriting an existing session replaces its tabs cleanly.
    let updated_default = Session::from_tabs(DEFAULT_SESSION_NAME, &tabs[1..], 0);
    store
        .save(&updated_default)
        .expect("overwrite default session");
    assert_eq!(
        store.load(DEFAULT_SESSION_NAME).expect("reload updated"),
        updated_default
    );

    let loaded_named = store.load("work-1").expect("load named session");
    assert_eq!(loaded_named, named_session);
    assert_eq!(
        store.list_names().expect("list session names"),
        vec!["default".to_string(), "work-1".to_string()]
    );

    if let Some(dir) = db_path.parent() {
        let _ = fs::remove_dir_all(dir);
    }
}

#[test]
fn missing_session_returns_not_found_and_optional_none() {
    let db_path = temp_db_path();
    let store = SessionStore::new(db_path.clone());

    assert!(matches!(
        store.load("missing"),
        Err(SessionError::NotFound(name)) if name == "missing"
    ));
    assert_eq!(
        store.load_optional("default").expect("optional missing"),
        None
    );

    if let Some(dir) = db_path.parent() {
        let _ = fs::remove_dir_all(dir);
    }
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

#[test]
fn resolve_db_path_uses_xdg_config_home_then_home() {
    assert_eq!(
        resolve_config_dir(Some("/tmp/xdg-config"), Some("/home/alice")),
        PathBuf::from("/tmp/xdg-config/nservo")
    );
    assert_eq!(
        resolve_db_path(Some("/tmp/xdg-config"), Some("/home/alice")),
        PathBuf::from("/tmp/xdg-config/nservo/nservo.db")
    );
    assert_eq!(
        resolve_db_path(Some(""), Some("/home/alice")),
        PathBuf::from("/home/alice/.config/nservo/nservo.db")
    );
    assert_eq!(
        resolve_db_path(None, Some("/home/alice")),
        PathBuf::from("/home/alice/.config/nservo/nservo.db")
    );
}
