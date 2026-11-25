use std::fmt;

/// The active interaction mode of the browser, modeled after Neovim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Input,
    Command,
    Hint,
}

impl Mode {
    /// Short uppercase label shown in the Neovim-style statusline badge.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Input => "INPUT",
            Self::Command => "COMMAND",
            Self::Hint => "HINT",
        }
    }

    /// CSS class applied to the statusline mode badge.
    pub const fn css_class(self) -> &'static str {
        match self {
            Self::Normal => "mode-normal",
            Self::Input => "mode-input",
            Self::Command => "mode-command",
            Self::Hint => "mode-hint",
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
