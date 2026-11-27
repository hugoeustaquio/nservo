use std::fs;
use std::path::Path;

use gtk::gdk;

use crate::core::default_style_path;

/// Built-in default GTK4 theme CSS loaded from `assets/style.css`.
pub const DEFAULT_THEME_CSS: &str = include_str!("../../assets/style.css");

/// Load the user's custom CSS from `custom_path` when present, otherwise fall
/// back to [`DEFAULT_THEME_CSS`].
pub fn resolve_theme_css(custom_path: &Path) -> String {
    if custom_path.is_file() {
        match fs::read_to_string(custom_path) {
            Ok(css) => return css,
            Err(err) => {
                log::warn!(
                    "Failed to read custom stylesheet {}: {err}; falling back to default theme",
                    custom_path.display()
                );
            }
        }
    }
    DEFAULT_THEME_CSS.to_string()
}

/// Load the active GTK4 theme CSS from `$XDG_CONFIG_HOME/nservo/style.css`
/// if it exists, or the built-in default stylesheet otherwise.
pub fn load_theme_css() -> String {
    resolve_theme_css(&default_style_path())
}

/// Install the active GTK4 CSS theme on the default GDK display.
pub fn install_theme() {
    let Some(display) = gdk::Display::default() else {
        return;
    };
    let css = load_theme_css();
    let provider = gtk::CssProvider::new();
    provider.load_from_string(&css);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
