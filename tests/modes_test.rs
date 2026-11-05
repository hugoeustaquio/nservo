use nservo::modes::{Mode, ModeStateMachine};

#[test]
fn test_mode_transitions() {
    let mut sm = ModeStateMachine::new();
    assert_eq!(sm.current(), Mode::Normal);

    // Enter insert mode with 'i'
    assert_eq!(sm.handle_key_input("i"), Some(Mode::Insert));
    assert_eq!(sm.current(), Mode::Insert);

    // Leave insert mode with 'ESC'
    assert_eq!(sm.handle_key_input("ESC"), Some(Mode::Normal));
    assert_eq!(sm.current(), Mode::Normal);

    // Enter command mode with ':'
    assert_eq!(sm.handle_key_input(":"), Some(Mode::Command));
    assert_eq!(sm.current(), Mode::Command);

    // Leave command mode with 'ESC'
    assert_eq!(sm.handle_key_input("ESC"), Some(Mode::Normal));
    assert_eq!(sm.current(), Mode::Normal);

    // Enter passthrough mode with 'ctrl+v'
    assert_eq!(sm.handle_key_input("ctrl+v"), Some(Mode::Passthrough));
    assert_eq!(sm.current(), Mode::Passthrough);

    // Leave passthrough mode
    assert_eq!(sm.handle_key_input("ESC"), Some(Mode::Normal));
    assert_eq!(sm.current(), Mode::Normal);
}
