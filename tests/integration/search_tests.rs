use std::time::Instant;

use nservo::core::{
    BrowserAction, KeyInput, KeyMapper, KeyOutcome, Mode, SearchEvent, SearchState,
    format_search_status, parse_command, parse_search_message,
};
use nservo::engine::search_js::{
    SEARCH_MESSAGE_HANDLER, clear_search_script, search_init_script, search_query_script,
    search_step_script,
};

#[test]
fn keymap_handles_slash_n_shift_n_and_search_mode() {
    let mut mapper = KeyMapper::new();
    let now = Instant::now();

    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('/'), now),
        KeyOutcome::Action(BrowserAction::EnterMode(Mode::Search))
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('n'), now),
        KeyOutcome::Action(BrowserAction::SearchNext)
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('N'), now),
        KeyOutcome::Action(BrowserAction::SearchPrev)
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Escape, now),
        KeyOutcome::Action(BrowserAction::ClearSearch)
    );
    assert_eq!(
        mapper.handle_key(Mode::Search, KeyInput::Char('a'), now),
        KeyOutcome::PassThrough
    );
    assert_eq!(
        mapper.handle_key(Mode::Search, KeyInput::Escape, now),
        KeyOutcome::Action(BrowserAction::EnterMode(Mode::Normal))
    );
}

#[test]
fn noh_commands_clear_search_highlights() {
    assert_eq!(parse_command(":noh"), Ok(BrowserAction::ClearSearch));
    assert_eq!(parse_command(":nohlsearch"), Ok(BrowserAction::ClearSearch));
}

#[test]
fn search_state_remembers_last_query_and_repeats_on_empty_submit() {
    let mut state = SearchState::new();
    assert_eq!(state.last_query(), None);
    assert_eq!(state.submit("   "), None);

    assert_eq!(state.submit("rust"), Some("rust".to_string()));
    assert_eq!(state.last_query(), Some("rust"));
    assert!(state.has_active_highlights());

    assert!(state.clear_highlights());
    assert!(!state.has_active_highlights());
    assert!(!state.clear_highlights());
    assert_eq!(state.last_query(), Some("rust"));

    // Empty submit repeats the previous pattern (Vim behavior).
    assert_eq!(state.submit(""), Some("rust".to_string()));
    assert!(state.has_active_highlights());
}

#[test]
fn parses_search_messages_and_formats_status() {
    let event = parse_search_message(
        r#"{"type":"results","query":"servo","current":2,"total":5,"wrapped":false}"#,
    );
    assert_eq!(
        event,
        Some(SearchEvent::Results {
            query: "servo".to_string(),
            current: 2,
            total: 5,
            wrapped: false,
        })
    );
    assert_eq!(
        parse_search_message(r#"{"type":"cleared"}"#),
        Some(SearchEvent::Cleared)
    );
    assert_eq!(parse_search_message("invalid"), None);

    assert_eq!(
        format_search_status("servo", 2, 5, false),
        ("/servo [2/5]".to_string(), false)
    );
    assert_eq!(
        format_search_status("servo", 1, 5, true),
        ("Search wrapped — /servo [1/5]".to_string(), false)
    );
    assert_eq!(
        format_search_status("missing", 0, 0, false),
        ("Pattern not found: missing".to_string(), true)
    );
}

#[test]
fn search_scripts_encode_queries_safely_and_wire_handler() {
    let init = search_init_script();
    assert!(init.contains(SEARCH_MESSAGE_HANDLER));
    assert!(init.contains("__nservoSearch"));
    assert!(init.contains("data-nservo-search"));

    let query_js = search_query_script(r#"a"b\c"#);
    assert!(query_js.contains(r#"__nservoSearch.search("a\"b\\c", false)"#));

    let next_js = search_step_script(1, "hello");
    assert!(next_js.contains(r#"__nservoSearch.step(1, "hello")"#));

    let prev_js = search_step_script(-1, "hello");
    assert!(prev_js.contains(r#"__nservoSearch.step(-1, "hello")"#));

    assert!(clear_search_script().contains("__nservoSearch.clear()"));
}
