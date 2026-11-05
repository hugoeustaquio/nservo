use nservo::session::SessionStorage;
use nservo::tabs::TabManager;

#[test]
fn test_session_save_and_restore() {
    let tmp_file = std::env::temp_dir().join("nservo_test_session.json");
    let storage = SessionStorage::new(tmp_file.clone());

    let mut tab_mgr = TabManager::new();
    tab_mgr.open_tab("https://servo.org").unwrap();
    tab_mgr.open_tab("https://rust-lang.org").unwrap();
    if let Some(tab) = tab_mgr.current_tab_mut() {
        tab.set_scroll(100, 250);
    }

    storage.save_from_tabs(&tab_mgr).unwrap();

    let mut restored_mgr = TabManager::new();
    let restored = storage.restore_into_tabs(&mut restored_mgr).unwrap();
    assert!(restored);
    assert_eq!(restored_mgr.tabs().len(), 2);
    assert_eq!(restored_mgr.tabs()[0].url, "https://servo.org");
    assert_eq!(restored_mgr.tabs()[1].url, "https://rust-lang.org");
    assert_eq!(restored_mgr.tabs()[1].get_scroll(), (100, 250));

    let _ = std::fs::remove_file(tmp_file);
}
