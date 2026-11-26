use gtk::gdk;

const THEME_CSS: &str = r#"
window.nservo-window {
    background-color: #1a1b26;
    color: #c0caf5;
    font-family: "JetBrains Mono", "Fira Code", monospace;
}

.tab-bar {
    background-color: #16161e;
    border-bottom: 1px solid #292e42;
    padding: 2px 4px;
}

.tab-button {
    background-color: #1f2335;
    color: #787c99;
    border: none;
    border-radius: 3px;
    padding: 3px 10px;
    min-height: 22px;
    font-size: 12px;
}

.tab-button:hover {
    background-color: #292e42;
    color: #c0caf5;
}

.tab-button.tab-active {
    background-color: #3b4261;
    color: #c0caf5;
    font-weight: bold;
}

.status-bar {
    background-color: #1f2335;
    border-top: 1px solid #292e42;
    padding: 0;
    min-height: 26px;
    font-size: 12px;
}

.mode-badge {
    padding: 3px 10px;
    font-weight: bold;
    color: #16161e;
}

.mode-badge.mode-normal {
    background-color: #7aa2f7;
}

.mode-badge.mode-insert {
    background-color: #9ece6a;
}

.mode-badge.mode-command {
    background-color: #e0af68;
}

.mode-badge.mode-hint {
    background-color: #bb9af7;
}

.status-url {
    color: #c0caf5;
    padding: 0 10px;
}

.status-url.error {
    color: #f7768e;
}

.status-pending {
    color: #e0af68;
    font-weight: bold;
    padding: 0 8px;
}

.status-meta {
    color: #a9b1d6;
    background-color: #292e42;
    padding: 3px 10px;
}

.command-bar {
    background-color: #16161e;
    padding: 2px 8px;
    min-height: 26px;
    font-size: 12px;
}

.command-prefix {
    color: #e0af68;
    font-weight: bold;
}

.command-entry {
    background-color: transparent;
    color: #c0caf5;
    border: none;
    box-shadow: none;
    padding: 0 4px;
    min-height: 22px;
}

.inspector-panel {
    background-color: #16161e;
    border-top: 2px solid #292e42;
}

.inspector-header {
    background-color: #1f2335;
    border-bottom: 1px solid #292e42;
    padding: 2px 6px;
}

.inspector-btn {
    background-color: #24283b;
    color: #a9b1d6;
    border: none;
    border-radius: 3px;
    padding: 2px 8px;
    min-height: 20px;
    font-size: 11px;
}

.inspector-btn:hover {
    background-color: #292e42;
    color: #c0caf5;
}

.inspector-btn.inspector-tab-active {
    background-color: #3b4261;
    color: #7aa2f7;
    font-weight: bold;
}

.inspector-text,
.inspector-text text {
    background-color: #16161e;
    color: #c0caf5;
    font-size: 12px;
    padding: 4px 6px;
}

.inspector-repl {
    background-color: #1a1b26;
    border-top: 1px solid #292e42;
    padding: 2px 8px;
    font-size: 12px;
}

.inspector-dom-list {
    background-color: #16161e;
    color: #c0caf5;
    font-size: 12px;
}

.inspector-dom-row {
    padding: 2px 6px;
}

scrollbar.page-scrollbar {
    background-color: transparent;
    border: none;
    opacity: 0;
    transition: opacity 220ms ease;
}

scrollbar.page-scrollbar.visible {
    opacity: 1;
}

scrollbar.page-scrollbar trough {
    background-color: transparent;
    border: none;
    padding: 2px;
    transition: background-color 150ms ease;
}

scrollbar.page-scrollbar.hovering trough {
    background-color: rgba(22, 22, 30, 0.55);
}

scrollbar.page-scrollbar.vertical slider {
    min-width: 5px;
    min-height: 36px;
    border-radius: 99px;
    background-color: rgba(122, 162, 247, 0.45);
    transition: min-width 150ms ease, background-color 150ms ease;
}

scrollbar.page-scrollbar.vertical.hovering slider {
    min-width: 10px;
    background-color: rgba(122, 162, 247, 0.85);
}

scrollbar.page-scrollbar.horizontal slider {
    min-height: 5px;
    min-width: 36px;
    border-radius: 99px;
    background-color: rgba(122, 162, 247, 0.45);
    transition: min-height 150ms ease, background-color 150ms ease;
}

scrollbar.page-scrollbar.horizontal.hovering slider {
    min-height: 10px;
    background-color: rgba(122, 162, 247, 0.85);
}

scrollbar.page-scrollbar slider:active {
    background-color: #bb9af7;
}
"#;

/// Install the Neovim-inspired dark CSS theme on the default GDK display.
pub fn install_theme() {
    let Some(display) = gdk::Display::default() else {
        return;
    };
    let provider = gtk::CssProvider::new();
    provider.load_from_string(THEME_CSS);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
