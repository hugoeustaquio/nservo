use nservo::core::TabModel;

#[test]
fn tabs_wrap_around_on_next_and_prev() {
    let mut model = TabModel::new();
    let first = model.add_tab("https://one.test", "One");
    let second = model.add_tab("https://two.test", "Two");
    let third = model.add_tab("https://three.test", "Three");

    // Newly added tab is active.
    assert_eq!(model.active_tab().map(|t| t.id), Some(third));

    // gt wraps from last -> first.
    assert_eq!(model.next_tab(), Some(first));
    assert_eq!(model.next_tab(), Some(second));
    assert_eq!(model.next_tab(), Some(third));

    // gT wraps from first -> last.
    assert_eq!(model.prev_tab(), Some(second));
    assert_eq!(model.prev_tab(), Some(first));
    assert_eq!(model.prev_tab(), Some(third));
}

#[test]
fn closing_tabs_updates_active_index_safely() {
    let mut model = TabModel::new();
    let first = model.add_tab("https://one.test", "One");
    let second = model.add_tab("https://two.test", "Two");

    assert_eq!(model.close_active(), Some(second));
    assert_eq!(model.len(), 1);
    assert_eq!(model.active_tab().map(|t| t.id), Some(first));

    assert_eq!(model.close_active(), Some(first));
    assert!(model.is_empty());
    assert_eq!(model.next_tab(), None);
    assert_eq!(model.prev_tab(), None);
}

#[test]
fn metadata_updates_reflect_in_display_title() {
    let mut model = TabModel::new();
    let id = model.add_tab("https://example.com", "");
    assert_eq!(
        model.active_tab().unwrap().display_title(),
        "https://example.com"
    );

    assert!(!model.update_url(id, "https://example.com".to_string()));
    assert!(model.update_url(id, "https://example.org".to_string()));

    model.update_title(id, "Example Domain".to_string());
    model.set_loading(id, true);
    let tab = model.active_tab().unwrap();
    assert_eq!(tab.display_title(), "Example Domain");
    assert!(tab.is_loading);
}

#[test]
fn closed_tabs_can_be_reopened_in_lifo_order_at_previous_index() {
    let mut model = TabModel::new();
    let _first = model.add_tab("https://one.test", "One");
    let _second = model.add_tab("https://two.test", "Two");
    let _third = model.add_tab("https://three.test", "Three");

    // Close middle tab (index 1), then last tab (new index 1).
    model.select_index(1);
    assert!(model.close_active().is_some());
    assert!(model.close_active().is_some());
    assert_eq!(model.len(), 1);

    let last = model.pop_closed().expect("last closed tab");
    assert_eq!(last.url, "https://three.test");
    assert_eq!(last.title, "Three");
    assert_eq!(last.index, 1);
    model.insert_tab_at(last.index, last.url, last.title);

    let prev = model.pop_closed().expect("previous closed tab");
    assert_eq!(prev.url, "https://two.test");
    assert_eq!(prev.index, 1);
    model.insert_tab_at(prev.index, prev.url, prev.title);

    let urls: Vec<&str> = model.tabs().iter().map(|t| t.url.as_str()).collect();
    assert_eq!(
        urls,
        vec!["https://one.test", "https://two.test", "https://three.test"]
    );
    assert_eq!(model.pop_closed(), None);
}
