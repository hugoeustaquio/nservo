use nservo::history::HistoryDatabase;

#[test]
fn test_history_crud_and_ranking() {
    let db = HistoryDatabase::open_in_memory().unwrap();

    db.record_visit("https://rust-lang.org", "Rust Programming Language").unwrap();
    db.record_visit("https://servo.org", "Servo Browser Engine").unwrap();
    // Second visit increments counter
    db.record_visit("https://servo.org", "Servo Browser Engine").unwrap();

    let results = db.search("servo", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].url, "https://servo.org");
    assert_eq!(results[0].visit_count, 2);

    let frequent = db.get_frequent_and_recent(5).unwrap();
    assert_eq!(frequent[0].url, "https://servo.org");

    db.clear().unwrap();
    assert_eq!(db.search("servo", 10).unwrap().len(), 0);
}
