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

    /// Link/element hint mode actions (`f` current tab, `F` new tab).
    StartHints { new_tab: bool },
    HintChar(char),
    HintBackspace,

    /// In-page Vim-style search actions (`/`, `n`, `N`, `:noh`).
    SearchNext,
    SearchPrev,
    ClearSearch,

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

    /// Viewport scrolling actions (directional motions carry a repeat count, e.g. `12j`).
    ScrollDown(u32),
    ScrollUp(u32),
    ScrollLeft(u32),
    ScrollRight(u32),
    ScrollTop,
    ScrollBottom,

    /// Tab management actions.
    NextTab,
    PrevTab,
    OpenStartPageInNewTab,
    OpenInCurrentTab(String),
    OpenInNewTab(String),
    CloseTab,
    ReopenClosedTab,

    /// Page navigation, zoom, and built-in inspector actions.
    Reload,
    GoBack,
    GoForward,
    SetZoom(f64),
    ZoomIn,
    ZoomOut,
    ShowHelp,
    ShowConfig,
    ToggleInspector,

    /// Session, bookmark & history persistence actions.
    SaveSession(String),
    SaveSessionAndQuit(String),
    LoadSession(String),
    AddBookmark,
    ShowBookmarks,
    RemoveBookmark(Option<String>),
    ShowHistory(Option<String>),
    ClearHistory,

    /// Adblock filter-list management actions (`:adblock <URL>`, `:adblock-update`).
    AddAdblockList(String),
    UpdateAdblockLists,

    /// Exit the browser application.
    Quit,
}

/// Default zoom increment/decrement step (`10%`) for `Ctrl++` and `Ctrl+-`.
pub const ZOOM_STEP: f64 = 0.1;

/// Adjust `current` zoom by `delta`, rounding cleanly to 1% steps within `0.1..=10.0`.
pub fn step_zoom(current: f64, delta: f64) -> f64 {
    let next = ((current + delta) * 100.0).round() / 100.0;
    next.clamp(0.1, 10.0)
}
