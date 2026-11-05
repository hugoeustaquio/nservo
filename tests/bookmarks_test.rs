use nservo::bookmarks::BookmarksDatabase;

#[test]
fn test_bookmarks_crud() {
    let db = BookmarksDatabase::open_in_memory().unwrap();

    let tags = vec!["search".to_string(), "foss".to_string()];
    db.add("https://duckduckgo.com", "DuckDuckGo", &tags).unwrap();
    db.add("https://servo.org", "Servo Engine", &["browser".to_string()]).unwrap();

    let all = db.list_all().unwrap();
    assert_eq!(all.len(), 2);

    let search_res = db.search("duck").unwrap();
    assert_eq!(search_res.len(), 1);
    assert_eq!(search_res[0].url, "https://duckduckgo.com");
    assert!(search_res[0].tags.contains(&"foss".to_string()));

    let removed = db.remove("https://duckduckgo.com").unwrap();
    assert!(removed);
    assert_eq!(db.list_all().unwrap().len(), 1);

    db.clear().unwrap();
    assert_eq!(db.list_all().unwrap().len(), 0);
}
