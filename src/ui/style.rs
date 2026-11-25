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

.mode-badge.mode-input {
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
