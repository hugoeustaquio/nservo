use std::time::Instant;

use nservo::core::{
    BrowserAction, HintEvent, KeyInput, KeyMapper, KeyOutcome, Mode, generate_hint_labels,
    parse_hint_message,
};

#[test]
fn hint_labels_are_single_letters_up_to_26() {
    assert!(generate_hint_labels(0).is_empty());
    assert_eq!(generate_hint_labels(3), vec!["a", "b", "c"]);

    let labels_26 = generate_hint_labels(26);
    assert_eq!(labels_26.len(), 26);
    assert_eq!(labels_26.first().map(String::as_str), Some("a"));
    assert_eq!(labels_26.last().map(String::as_str), Some("z"));
}

#[test]
fn hint_labels_beyond_26_are_prefix_free_two_letter_codes() {
    let labels = generate_hint_labels(30);
    assert_eq!(labels.len(), 30);
    assert_eq!(labels[0], "aa");
    assert_eq!(labels[25], "az");
    assert_eq!(labels[26], "ba");
    assert_eq!(labels[29], "bd");

    // Prefix-free property: no label is a prefix of any other label.
    for (i, a) in labels.iter().enumerate() {
        for (j, b) in labels.iter().enumerate() {
            if i != j {
                assert!(!b.starts_with(a), "'{a}' must not be a prefix of '{b}'");
            }
        }
    }
}

#[test]
fn parse_hint_messages_handles_all_variants() {
    assert_eq!(
        parse_hint_message(r#"{"event":"shown","count":12}"#),
        Some(HintEvent::Shown { count: 12 })
    );
    assert_eq!(
        parse_hint_message(r#"{"event":"activated","isInput":true}"#),
        Some(HintEvent::Activated { is_input: true })
    );
    assert_eq!(
        parse_hint_message(r#"{"event":"activated","isInput":false}"#),
        Some(HintEvent::Activated { is_input: false })
    );
    assert_eq!(
        parse_hint_message(r#"{"event":"cancelled"}"#),
        Some(HintEvent::Cancelled)
    );
    assert_eq!(
        parse_hint_message(r#"{"event":"empty"}"#),
        Some(HintEvent::Empty)
    );
    assert_eq!(parse_hint_message("not-json"), None);
}

#[test]
fn keymap_handles_f_and_hint_mode_keys() {
    let mut mapper = KeyMapper::new();
    let now = Instant::now();

    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('f'), now),
        KeyOutcome::Action(BrowserAction::StartHints)
    );
    assert_eq!(
        mapper.handle_key(Mode::Hint, KeyInput::Char('c'), now),
        KeyOutcome::Action(BrowserAction::HintChar('c'))
    );
    assert_eq!(
        mapper.handle_key(Mode::Hint, KeyInput::Char('C'), now),
        KeyOutcome::Action(BrowserAction::HintChar('c'))
    );
    assert_eq!(
        mapper.handle_key(Mode::Hint, KeyInput::Backspace, now),
        KeyOutcome::Action(BrowserAction::HintBackspace)
    );
    assert_eq!(
        mapper.handle_key(Mode::Hint, KeyInput::Escape, now),
        KeyOutcome::Action(BrowserAction::EnterMode(Mode::Normal))
    );
    assert_eq!(
        mapper.handle_key(Mode::Hint, KeyInput::Char('5'), now),
        KeyOutcome::Consumed
    );
}
