use crate::modes::mode::Mode;

pub struct ModeStateMachine {
    current: Mode,
    previous: Mode,
}

impl ModeStateMachine {
    pub fn new() -> Self {
        Self {
            current: Mode::Normal,
            previous: Mode::Normal,
        }
    }

    pub fn current(&self) -> Mode {
        self.current
    }

    pub fn transition_to(&mut self, next: Mode) {
        if self.current != next {
            self.previous = self.current;
            self.current = next;
        }
    }

    pub fn handle_key_input(&mut self, key: &str) -> Option<Mode> {
        match self.current {
            Mode::Normal => {
                if key == "i" {
                    self.transition_to(Mode::Insert);
                    Some(Mode::Insert)
                } else if key == ":" {
                    self.transition_to(Mode::Command);
                    Some(Mode::Command)
                } else if key == "ctrl+v" {
                    self.transition_to(Mode::Passthrough);
                    Some(Mode::Passthrough)
                } else {
                    None
                }
            }
            Mode::Insert => {
                if key == "ESC" || key == "Escape" {
                    self.transition_to(Mode::Normal);
                    Some(Mode::Normal)
                } else {
                    None
                }
            }
            Mode::Passthrough => {
                if key == "ctrl+[" || key == "ESC" {
                    self.transition_to(Mode::Normal);
                    Some(Mode::Normal)
                } else {
                    None
                }
            }
            Mode::Command => {
                if key == "ESC" || key == "Escape" {
                    self.transition_to(Mode::Normal);
                    Some(Mode::Normal)
                } else {
                    None
                }
            }
        }
    }
}

impl Default for ModeStateMachine {
    fn default() -> Self {
        Self::new()
    }
}
