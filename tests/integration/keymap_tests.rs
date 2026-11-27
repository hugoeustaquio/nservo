use std::time::{Duration, Instant};

use nservo::core::{
    BrowserAction, COUNT_SEQUENCE_TIMEOUT, KEY_SEQUENCE_TIMEOUT, KeyInput, KeyMapper, KeyOutcome,
    MAX_KEY_COUNT, Mode,
};

#[test]
fn normal_mode_scroll_and_mode_switches() {
    let mut mapper = KeyMapper::new();
    let now = Instant::now();

    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('j'), now),
        KeyOutcome::Action(BrowserAction::ScrollDown(1))
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('k'), now),
        KeyOutcome::Action(BrowserAction::ScrollUp(1))
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('h'), now),
        KeyOutcome::Action(BrowserAction::ScrollLeft(1))
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('l'), now),
        KeyOutcome::Action(BrowserAction::ScrollRight(1))
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('G'), now),
        KeyOutcome::Action(BrowserAction::ScrollBottom)
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('d'), now),
        KeyOutcome::Action(BrowserAction::CloseTab)
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('u'), now),
        KeyOutcome::Action(BrowserAction::ReopenClosedTab)
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('o'), now),
        KeyOutcome::Action(BrowserAction::EnterCommandWith("open ".to_string()))
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('O'), now),
        KeyOutcome::Action(BrowserAction::EnterCommandWith("Open ".to_string()))
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('r'), now),
        KeyOutcome::Action(BrowserAction::Reload)
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Reload, now),
        KeyOutcome::Action(BrowserAction::Reload)
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('i'), now),
        KeyOutcome::Action(BrowserAction::EnterMode(Mode::Insert))
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char(':'), now),
        KeyOutcome::Action(BrowserAction::EnterMode(Mode::Command))
    );
}

#[test]
fn gt_and_g_shift_t_switch_tabs_within_400ms() {
    assert_eq!(KEY_SEQUENCE_TIMEOUT, Duration::from_millis(400));

    let mut mapper = KeyMapper::new();
    let t0 = Instant::now();

    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('g'), t0),
        KeyOutcome::Pending("g".to_string())
    );
    assert_eq!(
        mapper.handle_key(
            Mode::Normal,
            KeyInput::Char('t'),
            t0 + Duration::from_millis(399)
        ),
        KeyOutcome::Action(BrowserAction::NextTab)
    );

    let t1 = t0 + Duration::from_millis(600);
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('g'), t1),
        KeyOutcome::Pending("g".to_string())
    );
    assert_eq!(
        mapper.handle_key(
            Mode::Normal,
            KeyInput::Char('T'),
            t1 + Duration::from_millis(350)
        ),
        KeyOutcome::Action(BrowserAction::PrevTab)
    );
}

#[test]
fn gg_and_go_trigger_within_timeout() {
    let mut mapper = KeyMapper::new();
    let t0 = Instant::now();

    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('g'), t0),
        KeyOutcome::Pending("g".to_string())
    );
    assert_eq!(
        mapper.handle_key(
            Mode::Normal,
            KeyInput::Char('g'),
            t0 + Duration::from_millis(100)
        ),
        KeyOutcome::Action(BrowserAction::ScrollTop)
    );

    let t1 = t0 + Duration::from_millis(600);
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('g'), t1),
        KeyOutcome::Pending("g".to_string())
    );
    assert_eq!(
        mapper.handle_key(
            Mode::Normal,
            KeyInput::Char('o'),
            t1 + Duration::from_millis(200)
        ),
        KeyOutcome::Action(BrowserAction::EditCurrentUrl)
    );

    let t2 = t0 + Duration::from_millis(1200);
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('w'), t2),
        KeyOutcome::Pending("w".to_string())
    );
    assert_eq!(
        mapper.handle_key(
            Mode::Normal,
            KeyInput::Char('i'),
            t2 + Duration::from_millis(150)
        ),
        KeyOutcome::Action(BrowserAction::ToggleInspector)
    );
}

#[test]
fn pending_g_expires_after_400ms() {
    let mut mapper = KeyMapper::new();
    let t0 = Instant::now();

    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('g'), t0),
        KeyOutcome::Pending("g".to_string())
    );
    assert_eq!(
        mapper.pending_display(t0 + Duration::from_millis(399)),
        Some("g".to_string())
    );
    assert_eq!(
        mapper.pending_display(t0 + Duration::from_millis(400)),
        None
    );

    // At >= 400ms, pressing 't' must not trigger NextTab.
    assert_eq!(
        mapper.handle_key(
            Mode::Normal,
            KeyInput::Char('t'),
            t0 + Duration::from_millis(400)
        ),
        KeyOutcome::Consumed
    );

    // And pressing 'j' after expiration executes ScrollDown(1) normally.
    assert_eq!(
        mapper.handle_key(
            Mode::Normal,
            KeyInput::Char('j'),
            t0 + Duration::from_millis(450)
        ),
        KeyOutcome::Action(BrowserAction::ScrollDown(1))
    );
}

#[test]
fn numeric_count_prefixes_multiply_scroll_commands() {
    let mut mapper = KeyMapper::new();
    let t0 = Instant::now();

    assert_eq!(mapper.handle_key(Mode::Normal, KeyInput::Char('1'), t0), KeyOutcome::Pending("1".into()));
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('2'), t0 + Duration::from_millis(200)),
        KeyOutcome::Pending("12".into())
    );
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('j'), t0 + Duration::from_millis(400)),
        KeyOutcome::Action(BrowserAction::ScrollDown(12))
    );
    assert_eq!(mapper.pending_display(t0 + Duration::from_millis(400)), None);

    for (digits, key, expected) in [
        ("10", 'k', BrowserAction::ScrollUp(10)),
        ("3", 'h', BrowserAction::ScrollLeft(3)),
        ("4", 'l', BrowserAction::ScrollRight(4)),
    ] {
        for ch in digits.chars() {
            assert!(matches!(mapper.handle_key(Mode::Normal, KeyInput::Char(ch), t0), KeyOutcome::Pending(_)));
        }
        assert_eq!(mapper.handle_key(Mode::Normal, KeyInput::Char(key), t0), KeyOutcome::Action(expected));
    }

    // Leading `0` does not start a count.
    assert_eq!(mapper.handle_key(Mode::Normal, KeyInput::Char('0'), t0), KeyOutcome::Consumed);
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
    assert_eq!(mapper.handle_key(Mode::Normal, KeyInput::Char('2'), t0), KeyOutcome::Pending("2".into()));
    assert_eq!(mapper.handle_key(Mode::Normal, KeyInput::Char('5'), t0), KeyOutcome::Pending("25".into()));
    assert_eq!(mapper.handle_key(Mode::Normal, KeyInput::Backspace, t0), KeyOutcome::Pending("2".into()));
    assert_eq!(mapper.handle_key(Mode::Normal, KeyInput::Escape, t0), KeyOutcome::Consumed);
    assert_eq!(mapper.handle_key(Mode::Normal, KeyInput::Char('j'), t0), KeyOutcome::Action(BrowserAction::ScrollDown(1)));

    // Refreshes timeout on each digit and expires at COUNT_SEQUENCE_TIMEOUT.
    let _ = mapper.handle_key(Mode::Normal, KeyInput::Char('1'), t0);
    let t1 = t0 + COUNT_SEQUENCE_TIMEOUT - Duration::from_millis(100);
    assert_eq!(mapper.handle_key(Mode::Normal, KeyInput::Char('2'), t1), KeyOutcome::Pending("12".into()));
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('j'), t1 + Duration::from_millis(200)),
        KeyOutcome::Action(BrowserAction::ScrollDown(12))
    );

    let _ = mapper.handle_key(Mode::Normal, KeyInput::Char('7'), t0);
    assert_eq!(
        mapper.handle_key(Mode::Normal, KeyInput::Char('j'), t0 + COUNT_SEQUENCE_TIMEOUT),
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

#[test]
fn insert_and_command_modes_pass_through_except_escape() {
    let mut mapper = KeyMapper::new();
    let now = Instant::now();

    assert_eq!(
        mapper.handle_key(Mode::Insert, KeyInput::Char('j'), now),
        KeyOutcome::PassThrough
    );
    assert_eq!(
        mapper.handle_key(Mode::Insert, KeyInput::Escape, now),
        KeyOutcome::Action(BrowserAction::EnterMode(Mode::Normal))
    );

    assert_eq!(
        mapper.handle_key(Mode::Command, KeyInput::Char('o'), now),
        KeyOutcome::PassThrough
    );
    assert_eq!(
        mapper.handle_key(Mode::Command, KeyInput::Escape, now),
        KeyOutcome::Action(BrowserAction::EnterMode(Mode::Normal))
    );
}
