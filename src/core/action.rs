use crate::core::{LineEdge, Mode, WordDirection};

/// High-level browser actions produced by key bindings or Ex-style commands.
#[derive(Debug, Clone, PartialEq)]
pub enum BrowserAction {
    /// Switch to a different Neovim interaction mode.
    EnterMode(Mode),
    /// Enter Command mode with pre-filled command text (e.g. `"open "`, `"Open "`).
    EnterCommandWith(String),
    /// Enter Command mode pre-filled with `:open <current URL>` (`go`).
    EditCurrentUrl,

    /// Link/element hint mode actions (`f`).
    StartHints,
    HintChar(char),
    HintBackspace,

    /// Insert composed or Unicode/UTF-16 text into the focused page input.
    InsertText(String),
    /// Move or extend selection by one word in the focused page input (`Ctrl[+Shift]+Arrow`).
    MoveWord {
        direction: WordDirection,
        select: bool,
    },
    /// Move caret to the start (`Ctrl+A`) or end (`Ctrl+E`) of the current line.
    MoveLineEdge(LineEdge),

    /// System clipboard actions (`Ctrl+C` / `Ctrl+X` / `Ctrl+V`).
    CopySelection,
    CutSelection,
    PasteClipboard,

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
