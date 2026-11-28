use std::time::{Duration, Instant};

use nservo::core::{
    BrowserAction, COUNT_SEQUENCE_TIMEOUT, KeyInput, KeyMapper, KeyOutcome, MAX_KEY_COUNT, Mode,
};

#[test]
fn numeric_count_prefixes_multiply_scroll_commands() {
    let mut mapper = KeyMapper::new();
    let t0 = Instant::now();

    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('1'), t0),
        KeyOutcome::Pending("1".into())
    );
    assert_eq!(
        mapper.handle_key(
            Mode::Normal,
            KeyInput::Char('2'),
            t0 + Duration::from_millis(200)
        ),
        KeyOutcome::Pending("12".into())
    );
    assert_eq!(
        mapper.handle_key(
            Mode::Normal,
            KeyInput::Char('j'),
            t0 + Duration::from_millis(400)
        ),
        KeyOutcome::Action(BrowserAction::ScrollDown(12))
    );
    assert_eq!(
        mapper.pending_display(t0 + Duration::from_millis(400)),
        None
    );

    for (digits, key, expected) in [
        ("10", 'k', BrowserAction::ScrollUp(10)),
        ("3", 'h', BrowserAction::ScrollLeft(3)),
        ("4", 'l', BrowserAction::ScrollRight(4)),
    ] {
        for ch in digits.chars() {
            assert!(matches!(
                mapper.handle_key(Mode::Normal, KeyInput::Char(ch), t0),
                KeyOutcome::Pending(_)
            ));
        }
        assert_eq!(
            mapper.handle_key(Mode::Normal, KeyInput::Char(key), t0),
            KeyOutcome::Action(expected)
        );
    }

    // Leading `0` does not start a count.
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('0'), t0),
        KeyOutcome::Consumed
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('j'), t0),
        KeyOutcome::Action(BrowserAction::ScrollDown(1))
    );
}

#[test]
fn numeric_count_handles_backspace_escape_clamp_and_timeout() {
    let mut mapper = KeyMapper::new();
    let t0 = Instant::now();

    // Backspace edits digits; Escape cancels without firing ClearSearch.
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('2'), t0),
        KeyOutcome::Pending("2".into())
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('5'), t0),
        KeyOutcome::Pending("25".into())
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Backspace, t0),
        KeyOutcome::Pending("2".into())
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Escape, t0),
        KeyOutcome::Consumed
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('j'), t0),
        KeyOutcome::Action(BrowserAction::ScrollDown(1))
    );

    // Refreshes timeout on each digit and expires at COUNT_SEQUENCE_TIMEOUT.
    let _ = mapper.handle_key(Mode::Normal, KeyInput::Char('1'), t0);
    let t1 = t0 + COUNT_SEQUENCE_TIMEOUT - Duration::from_millis(100);
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('2'), t1),
        KeyOutcome::Pending("12".into())
    );
    assert_eq!(
        mapper.handle_key(
            Mode::Normal,
            KeyInput::Char('j'),
            t1 + Duration::from_millis(200)
        ),
        KeyOutcome::Action(BrowserAction::ScrollDown(12))
    );

    let _ = mapper.handle_key(Mode::Normal, KeyInput::Char('7'), t0);
    assert_eq!(
        mapper.handle_key(
            Mode::Normal,
            KeyInput::Char('j'),
            t0 + COUNT_SEQUENCE_TIMEOUT
        ),
        KeyOutcome::Action(BrowserAction::ScrollDown(1))
    );

    // Oversized counts clamp to MAX_KEY_COUNT.
    for _ in 0..8 {
        let _ = mapper.handle_key(Mode::Normal, KeyInput::Char('9'), t0);
    }
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('j'), t0),
        KeyOutcome::Action(BrowserAction::ScrollDown(MAX_KEY_COUNT))
    );
}
