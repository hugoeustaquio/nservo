use std::time::{Duration, Instant};

use nservo::core::{BrowserAction, KEY_SEQUENCE_TIMEOUT, KeyInput, KeyMapper, KeyOutcome, Mode};

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
