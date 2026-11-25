use crate::core::Mode;

/// High-level browser actions produced by key bindings or Ex-style commands.
#[derive(Debug, Clone, PartialEq)]
pub enum BrowserAction {
    /// Switch to a different Neovim interaction mode.
    EnterMode(Mode),
    /// Enter Command mode with pre-filled command text (e.g. `"open "`, `"Open "`).
    EnterCommandWith(String),

    /// Link/element hint mode actions (`f`).
    StartHints,
    HintChar(char),
    HintBackspace,

    /// Viewport scrolling actions.
    ScrollDown,
    ScrollUp,
    ScrollLeft,
    ScrollRight,
    ScrollTop,
    ScrollBottom,

    /// Tab management actions.
    NextTab,
    PrevTab,
    OpenInCurrentTab(String),
    OpenInNewTab(String),
    CloseTab,

    /// Page navigation and zoom actions.
    Reload,
    GoBack,
    GoForward,
    SetZoom(f64),
    ShowHelp,

    /// Session persistence actions.
    SaveSession(String),
    SaveSessionAndQuit(String),
    LoadSession(String),

    /// Exit the browser application.
    Quit,
}
