use std::fmt;

/// Operating modes of NServo, heavily inspired by Neovim and qutebrowser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Passthrough,
    Command,
}

impl Mode {
    pub fn label(&self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Passthrough => "PASSTHROUGH",
            Mode::Command => "COMMAND",
        }
    }

    pub fn intercepts_navigation_keys(&self) -> bool {
        match self {
            Mode::Normal => true,
            Mode::Insert => false,
            Mode::Passthrough => false,
            Mode::Command => false,
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}
