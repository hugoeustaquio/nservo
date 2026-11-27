use std::time::{Duration, Instant};

use crate::core::{BrowserAction, Mode};

/// Timeout window for multi-key Neovim sequences such as `gt`, `gT`, and `gg`.
pub const KEY_SEQUENCE_TIMEOUT: Duration = Duration::from_millis(400);

/// Timeout window for buffered numeric count prefixes such as `12j` or `5k`.
pub const COUNT_SEQUENCE_TIMEOUT: Duration = Duration::from_millis(1500);

/// Maximum repeat count accepted for a single counted command.
pub const MAX_KEY_COUNT: u32 = 9_999;

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
    /// Key started or extended a multi-key sequence / numeric count prefix.
    Pending(String),
    /// Key was consumed in Normal mode with no resulting action.
    Consumed,
    /// Key should propagate untouched to the focused widget (e.g. WebView or Entry).
    PassThrough,
}

#[derive(Debug, Clone, Copy)]
enum PendingKind {
    Prefix(char),
    Count(u32),
}

#[derive(Debug, Clone, Copy)]
struct PendingState {
    kind: PendingKind,
    started_at: Instant,
}

/// Pure, deterministic Neovim keybinding state machine with timeout support.
#[derive(Debug, Default)]
pub struct KeyMapper {
    pending: Option<PendingState>,
}

impl KeyMapper {
    pub fn new() -> Self {
        Self::default()
    }

    /// Currently buffered prefix key or numeric count (e.g. `"g"` or `"12"`), if not expired.
    pub fn pending_display(&self, now: Instant) -> Option<String> {
        match self.active_pending(now)? {
            PendingKind::Prefix(ch) => Some(ch.to_string()),
            PendingKind::Count(count) => Some(count.to_string()),
        }
    }

    /// Timeout duration associated with the currently buffered pending state, if any.
    pub fn pending_timeout(&self) -> Option<Duration> {
        match self.pending?.kind {
            PendingKind::Prefix(_) => Some(KEY_SEQUENCE_TIMEOUT),
            PendingKind::Count(_) => Some(COUNT_SEQUENCE_TIMEOUT),
        }
    }

    /// Clear any pending multi-key prefix or numeric count (for example when its timer fires).
    pub fn clear_pending(&mut self) {
        self.pending = None;
    }

    /// Process a key press in `mode` at timestamp `now`.
    pub fn handle_key(&mut self, mode: Mode, input: KeyInput, now: Instant) -> KeyOutcome {
        match mode {
            Mode::Normal => self.handle_normal(input, now),
            Mode::Insert => self.handle_insert(input),
            Mode::Command | Mode::Search => self.handle_prompt(input),
            Mode::Hint => self.handle_hint(input),
        }
    }

    fn active_pending(&self, now: Instant) -> Option<PendingKind> {
        let pending = self.pending?;
        let timeout = match pending.kind {
            PendingKind::Prefix(_) => KEY_SEQUENCE_TIMEOUT,
            PendingKind::Count(_) => COUNT_SEQUENCE_TIMEOUT,
        };
        if now.saturating_duration_since(pending.started_at) < timeout {
            Some(pending.kind)
        } else {
            None
        }
    }

    fn set_count(&mut self, count: u32, now: Instant) -> KeyOutcome {
        self.pending = Some(PendingState {
            kind: PendingKind::Count(count),
            started_at: now,
        });
        KeyOutcome::Pending(count.to_string())
    }

    fn handle_normal(&mut self, input: KeyInput, now: Instant) -> KeyOutcome {
        let count = match self.active_pending(now) {
            Some(PendingKind::Prefix(prefix)) => {
                self.pending = None;
                return self.resolve_sequence(prefix, input);
            }
            Some(PendingKind::Count(count)) => match input {
                KeyInput::Char(ch) if ch.is_ascii_digit() => {
                    let digit = ch.to_digit(10).unwrap_or(0);
                    let next = count
                        .saturating_mul(10)
                        .saturating_add(digit)
                        .min(MAX_KEY_COUNT);
                    return self.set_count(next, now);
                }
                KeyInput::Backspace => {
                    let next = count / 10;
                    if next > 0 {
                        return self.set_count(next, now);
                    }
                    self.pending = None;
                    return KeyOutcome::Consumed;
                }
                KeyInput::Escape => {
                    self.pending = None;
                    return KeyOutcome::Consumed;
                }
                _ => {
                    self.pending = None;
                    count
                }
            },
            None => {
                // Drop any expired prefix/count before handling the fresh key.
                self.pending = None;
                if let KeyInput::Char(ch @ '1'..='9') = input {
                    return self.set_count(ch.to_digit(10).unwrap_or(1), now);
                }
                1
            }
        };

        match input {
            KeyInput::Escape => KeyOutcome::Action(BrowserAction::ClearSearch),
            KeyInput::Char(prefix @ ('g' | 'w')) => {
                self.pending = Some(PendingState {
                    kind: PendingKind::Prefix(prefix),
                    started_at: now,
                });
                KeyOutcome::Pending(prefix.to_string())
            }
            KeyInput::Char('j') => KeyOutcome::Action(BrowserAction::ScrollDown(count)),
            KeyInput::Char('k') => KeyOutcome::Action(BrowserAction::ScrollUp(count)),
            KeyInput::Char('h') => KeyOutcome::Action(BrowserAction::ScrollLeft(count)),
            KeyInput::Char('l') => KeyOutcome::Action(BrowserAction::ScrollRight(count)),
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
            KeyInput::Char('f') => KeyOutcome::Action(BrowserAction::StartHints { new_tab: false }),
            KeyInput::Char('F') => KeyOutcome::Action(BrowserAction::StartHints { new_tab: true }),
            KeyInput::Char('b') => KeyOutcome::Action(BrowserAction::AddBookmark),
            KeyInput::Char('B') => KeyOutcome::Action(BrowserAction::ShowBookmarks),
            KeyInput::Char('/') => KeyOutcome::Action(BrowserAction::EnterMode(Mode::Search)),
            KeyInput::Char('n') => KeyOutcome::Action(BrowserAction::SearchNext),
            KeyInput::Char('N') => KeyOutcome::Action(BrowserAction::SearchPrev),
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
            ('g', KeyInput::Char('h')) => KeyOutcome::Action(BrowserAction::ShowHistory(None)),
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

    fn handle_prompt(&mut self, input: KeyInput) -> KeyOutcome {
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
