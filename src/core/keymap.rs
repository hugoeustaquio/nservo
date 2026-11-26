use std::time::{Duration, Instant};

use crate::core::{BrowserAction, Mode};

/// Timeout window for multi-key Neovim sequences such as `gt`, `gT`, and `gg`.
pub const KEY_SEQUENCE_TIMEOUT: Duration = Duration::from_millis(400);

/// Mode-independent keyboard input fed into [`KeyMapper`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyInput {
    Escape,
    Backspace,
    Reload,
    Char(char),
    Other,
}

/// Result of processing a single key press in the current [`Mode`].
#[derive(Debug, Clone, PartialEq)]
pub enum KeyOutcome {
    /// Key completed a command and triggered a [`BrowserAction`].
    Action(BrowserAction),
    /// Key started a multi-key sequence; wait up to [`KEY_SEQUENCE_TIMEOUT`].
    Pending(String),
    /// Key was consumed in Normal mode with no resulting action.
    Consumed,
    /// Key should propagate untouched to the focused widget (e.g. WebView or Entry).
    PassThrough,
}

#[derive(Debug, Clone, Copy)]
struct PendingPrefix {
    key: char,
    started_at: Instant,
}

/// Pure, deterministic Neovim keybinding state machine with a 400ms timeout.
#[derive(Debug, Default)]
pub struct KeyMapper {
    pending: Option<PendingPrefix>,
}

impl KeyMapper {
    pub fn new() -> Self {
        Self::default()
    }

    /// Currently buffered prefix key (e.g. `"g"`), if any and not yet expired.
    pub fn pending_display(&self, now: Instant) -> Option<String> {
        self.active_prefix(now).map(|ch| ch.to_string())
    }

    /// Clear any pending multi-key prefix (for example when the 400ms timer fires).
    pub fn clear_pending(&mut self) {
        self.pending = None;
    }

    /// Process a key press in `mode` at timestamp `now`.
    pub fn handle_key(&mut self, mode: Mode, input: KeyInput, now: Instant) -> KeyOutcome {
        match mode {
            Mode::Normal => self.handle_normal(input, now),
            Mode::Insert => self.handle_insert(input),
            Mode::Command => self.handle_command(input),
            Mode::Hint => self.handle_hint(input),
        }
    }

    fn active_prefix(&self, now: Instant) -> Option<char> {
        let pending = self.pending?;
        if now.saturating_duration_since(pending.started_at) < KEY_SEQUENCE_TIMEOUT {
            Some(pending.key)
        } else {
            None
        }
    }

    fn handle_normal(&mut self, input: KeyInput, now: Instant) -> KeyOutcome {
        if let Some(prefix) = self.active_prefix(now) {
            self.pending = None;
            return self.resolve_sequence(prefix, input);
        }

        // Drop any expired prefix before handling the fresh key.
        self.pending = None;

        match input {
            KeyInput::Escape => KeyOutcome::Consumed,
            KeyInput::Char(prefix @ ('g' | 'w')) => {
                self.pending = Some(PendingPrefix {
                    key: prefix,
                    started_at: now,
                });
                KeyOutcome::Pending(prefix.to_string())
            }
            KeyInput::Char('j') => KeyOutcome::Action(BrowserAction::ScrollDown),
            KeyInput::Char('k') => KeyOutcome::Action(BrowserAction::ScrollUp),
            KeyInput::Char('h') => KeyOutcome::Action(BrowserAction::ScrollLeft),
            KeyInput::Char('l') => KeyOutcome::Action(BrowserAction::ScrollRight),
            KeyInput::Char('G') => KeyOutcome::Action(BrowserAction::ScrollBottom),
            KeyInput::Char('H') => KeyOutcome::Action(BrowserAction::GoBack),
            KeyInput::Char('L') => KeyOutcome::Action(BrowserAction::GoForward),
            KeyInput::Char('d') => KeyOutcome::Action(BrowserAction::CloseTab),
            KeyInput::Char('u') => KeyOutcome::Action(BrowserAction::ReopenClosedTab),
            KeyInput::Char('o') => {
                KeyOutcome::Action(BrowserAction::EnterCommandWith("open ".to_string()))
            }
            KeyInput::Char('O') => {
                KeyOutcome::Action(BrowserAction::EnterCommandWith("Open ".to_string()))
            }
            KeyInput::Char('r') | KeyInput::Reload => KeyOutcome::Action(BrowserAction::Reload),
            KeyInput::Char('f') => KeyOutcome::Action(BrowserAction::StartHints),
            KeyInput::Char('i') => KeyOutcome::Action(BrowserAction::EnterMode(Mode::Insert)),
            KeyInput::Char(':') => KeyOutcome::Action(BrowserAction::EnterMode(Mode::Command)),
            KeyInput::Char(_) | KeyInput::Backspace | KeyInput::Other => KeyOutcome::Consumed,
        }
    }

    fn resolve_sequence(&self, prefix: char, second: KeyInput) -> KeyOutcome {
        match (prefix, second) {
            ('g', KeyInput::Char('t')) => KeyOutcome::Action(BrowserAction::NextTab),
            ('g', KeyInput::Char('T')) => KeyOutcome::Action(BrowserAction::PrevTab),
            ('g', KeyInput::Char('g')) => KeyOutcome::Action(BrowserAction::ScrollTop),
            ('g', KeyInput::Char('o')) => KeyOutcome::Action(BrowserAction::EditCurrentUrl),
            ('w', KeyInput::Char('i')) => KeyOutcome::Action(BrowserAction::ToggleInspector),
            _ => KeyOutcome::Consumed,
        }
    }

    fn handle_insert(&mut self, input: KeyInput) -> KeyOutcome {
        self.pending = None;
        match input {
            KeyInput::Escape => KeyOutcome::Action(BrowserAction::EnterMode(Mode::Normal)),
            _ => KeyOutcome::PassThrough,
        }
    }

    fn handle_command(&mut self, input: KeyInput) -> KeyOutcome {
        self.pending = None;
        match input {
            KeyInput::Escape => KeyOutcome::Action(BrowserAction::EnterMode(Mode::Normal)),
            _ => KeyOutcome::PassThrough,
        }
    }

    fn handle_hint(&mut self, input: KeyInput) -> KeyOutcome {
        self.pending = None;
        match input {
            KeyInput::Escape => KeyOutcome::Action(BrowserAction::EnterMode(Mode::Normal)),
            KeyInput::Backspace => KeyOutcome::Action(BrowserAction::HintBackspace),
            KeyInput::Char(ch) if ch.is_ascii_alphabetic() => {
                KeyOutcome::Action(BrowserAction::HintChar(ch.to_ascii_lowercase()))
            }
            _ => KeyOutcome::Consumed,
        }
    }
}
