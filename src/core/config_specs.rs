use crate::core::{BrowserAction, Mode};

/// Metadata describing one customizable Normal-mode keyboard shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutSpec {
    pub id: &'static str,
    pub default_key: &'static str,
    pub category: &'static str,
    pub description: &'static str,
}

/// Canonical registry of all customizable Normal-mode shortcuts.
pub const SHORTCUT_SPECS: &[ShortcutSpec] = &[
    ShortcutSpec {
        id: "scroll_down",
        default_key: "j",
        category: "Navigation",
        description: "Scroll down (supports numeric count)",
    },
    ShortcutSpec {
        id: "scroll_up",
        default_key: "k",
        category: "Navigation",
        description: "Scroll up (supports numeric count)",
    },
    ShortcutSpec {
        id: "scroll_left",
        default_key: "h",
        category: "Navigation",
        description: "Scroll left (supports numeric count)",
    },
    ShortcutSpec {
        id: "scroll_right",
        default_key: "l",
        category: "Navigation",
        description: "Scroll right (supports numeric count)",
    },
    ShortcutSpec {
        id: "scroll_top",
        default_key: "gg",
        category: "Navigation",
        description: "Scroll to top of page",
    },
    ShortcutSpec {
        id: "scroll_bottom",
        default_key: "G",
        category: "Navigation",
        description: "Scroll to bottom of page",
    },
    ShortcutSpec {
        id: "go_back",
        default_key: "H",
        category: "Navigation",
        description: "Navigate back in history",
    },
    ShortcutSpec {
        id: "go_forward",
        default_key: "L",
        category: "Navigation",
        description: "Navigate forward in history",
    },
    ShortcutSpec {
        id: "reload",
        default_key: "r",
        category: "Navigation",
        description: "Reload current tab",
    },
    ShortcutSpec {
        id: "next_tab",
        default_key: "gt",
        category: "Tabs",
        description: "Switch to next tab",
    },
    ShortcutSpec {
        id: "prev_tab",
        default_key: "gT",
        category: "Tabs",
        description: "Switch to previous tab",
    },
    ShortcutSpec {
        id: "close_tab",
        default_key: "d",
        category: "Tabs",
        description: "Close current tab",
    },
    ShortcutSpec {
        id: "reopen_tab",
        default_key: "u",
        category: "Tabs",
        description: "Reopen last closed tab",
    },
    ShortcutSpec {
        id: "open_prompt",
        default_key: "o",
        category: "Commands & Modes",
        description: "Open command bar with :open",
    },
    ShortcutSpec {
        id: "open_new_tab_prompt",
        default_key: "O",
        category: "Commands & Modes",
        description: "Open command bar with :Open",
    },
    ShortcutSpec {
        id: "edit_url",
        default_key: "go",
        category: "Commands & Modes",
        description: "Edit current tab URL in command bar",
    },
    ShortcutSpec {
        id: "command_mode",
        default_key: ":",
        category: "Commands & Modes",
        description: "Enter Command mode",
    },
    ShortcutSpec {
        id: "insert_mode",
        default_key: "i",
        category: "Commands & Modes",
        description: "Enter Insert mode",
    },
    ShortcutSpec {
        id: "hints",
        default_key: "f",
        category: "Hints & Search",
        description: "Follow links/inputs with hints",
    },
    ShortcutSpec {
        id: "hints_new_tab",
        default_key: "F",
        category: "Hints & Search",
        description: "Open link in new tab with hints",
    },
    ShortcutSpec {
        id: "search",
        default_key: "/",
        category: "Hints & Search",
        description: "Search text in current page",
    },
    ShortcutSpec {
        id: "search_next",
        default_key: "n",
        category: "Hints & Search",
        description: "Jump to next search match",
    },
    ShortcutSpec {
        id: "search_prev",
        default_key: "N",
        category: "Hints & Search",
        description: "Jump to previous search match",
    },
    ShortcutSpec {
        id: "add_bookmark",
        default_key: "b",
        category: "Library & Tools",
        description: "Bookmark current page",
    },
    ShortcutSpec {
        id: "show_bookmarks",
        default_key: "B",
        category: "Library & Tools",
        description: "Open bookmarks page (about:bookmarks)",
    },
    ShortcutSpec {
        id: "show_history",
        default_key: "gh",
        category: "Library & Tools",
        description: "Open browsing history (about:history)",
    },
    ShortcutSpec {
        id: "toggle_inspector",
        default_key: "wi",
        category: "Library & Tools",
        description: "Toggle Web Inspector drawer",
    },
];

pub(crate) fn action_for_id(id: &str, count: u32) -> Option<BrowserAction> {
    Some(match id {
        "scroll_down" => BrowserAction::ScrollDown(count),
        "scroll_up" => BrowserAction::ScrollUp(count),
        "scroll_left" => BrowserAction::ScrollLeft(count),
        "scroll_right" => BrowserAction::ScrollRight(count),
        "scroll_top" => BrowserAction::ScrollTop,
        "scroll_bottom" => BrowserAction::ScrollBottom,
        "go_back" => BrowserAction::GoBack,
        "go_forward" => BrowserAction::GoForward,
        "reload" => BrowserAction::Reload,
        "next_tab" => BrowserAction::NextTab,
        "prev_tab" => BrowserAction::PrevTab,
        "close_tab" => BrowserAction::CloseTab,
        "reopen_tab" => BrowserAction::ReopenClosedTab,
        "open_prompt" => BrowserAction::EnterCommandWith("open ".to_string()),
        "open_new_tab_prompt" => BrowserAction::EnterCommandWith("Open ".to_string()),
        "edit_url" => BrowserAction::EditCurrentUrl,
        "command_mode" => BrowserAction::EnterMode(Mode::Command),
        "insert_mode" => BrowserAction::EnterMode(Mode::Insert),
        "hints" => BrowserAction::StartHints { new_tab: false },
        "hints_new_tab" => BrowserAction::StartHints { new_tab: true },
        "search" => BrowserAction::EnterMode(Mode::Search),
        "search_next" => BrowserAction::SearchNext,
        "search_prev" => BrowserAction::SearchPrev,
        "add_bookmark" => BrowserAction::AddBookmark,
        "show_bookmarks" => BrowserAction::ShowBookmarks,
        "show_history" => BrowserAction::ShowHistory(None),
        "toggle_inspector" => BrowserAction::ToggleInspector,
        _ => return None,
    })
}
