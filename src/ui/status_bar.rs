use gtk::pango;
use gtk::prelude::*;

use crate::core::{Mode, TabInfo};
use crate::engine::gpu::RenderBackend;

/// Snapshot of status information rendered on the Neovim-style statusline.
pub struct StatusSnapshot<'a> {
    pub mode: Mode,
    pub tab: Option<&'a TabInfo>,
    pub active_index: usize,
    pub tab_count: usize,
    pub zoom_level: f64,
    pub pending_keys: Option<&'a str>,
    pub render_backend: RenderBackend,
}

/// Bottom Neovim-style statusline showing mode badge, active URL/title,
/// pending key sequence, tab position, and zoom level.
#[derive(Clone)]
pub struct StatusBar {
    container: gtk::Box,
    mode_label: gtk::Label,
    location_label: gtk::Label,
    pending_label: gtk::Label,
    meta_label: gtk::Label,
}

impl StatusBar {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        container.add_css_class("status-bar");
        container.set_hexpand(true);

        let mode_label = gtk::Label::new(Some(Mode::Normal.label()));
        mode_label.add_css_class("mode-badge");
        mode_label.add_css_class(Mode::Normal.css_class());

        let location_label = gtk::Label::new(None);
        location_label.add_css_class("status-url");
        location_label.set_hexpand(true);
        location_label.set_halign(gtk::Align::Fill);
        location_label.set_xalign(0.0);
        location_label.set_ellipsize(pango::EllipsizeMode::End);

        let pending_label = gtk::Label::new(None);
        pending_label.add_css_class("status-pending");

        let meta_label = gtk::Label::new(None);
        meta_label.add_css_class("status-meta");

        container.append(&mode_label);
        container.append(&location_label);
        container.append(&pending_label);
        container.append(&meta_label);

        Self {
            container,
            mode_label,
            location_label,
            pending_label,
            meta_label,
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }

    pub fn update(&self, snapshot: &StatusSnapshot<'_>) {
        self.update_mode_badge(snapshot.mode);
        self.location_label.remove_css_class("error");
        self.location_label.set_text(&format_location(snapshot.tab));
        self.pending_label
            .set_text(snapshot.pending_keys.unwrap_or(""));

        let tab_pos = if snapshot.tab_count == 0 {
            "0/0".to_string()
        } else {
            format!("{}/{}", snapshot.active_index + 1, snapshot.tab_count)
        };
        let zoom_pct = (snapshot.zoom_level * 100.0).round() as i32;
        let backend = snapshot.render_backend.short_label();
        self.meta_label
            .set_text(&format!("TAB {tab_pos}  |  ZOOM {zoom_pct}%  |  {backend}"));
    }

    pub fn show_notice(&self, text: &str) {
        self.location_label.remove_css_class("error");
        self.location_label.set_text(text);
    }

    pub fn show_error(&self, text: &str) {
        self.location_label.add_css_class("error");
        self.location_label.set_text(text);
    }

    pub fn set_pending_keys(&self, pending: Option<&str>) {
        self.pending_label.set_text(pending.unwrap_or(""));
    }

    fn update_mode_badge(&self, mode: Mode) {
        self.mode_label.set_text(mode.label());
        for candidate in [Mode::Normal, Mode::Insert, Mode::Command, Mode::Hint] {
            self.mode_label.remove_css_class(candidate.css_class());
        }
        self.mode_label.add_css_class(mode.css_class());
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

fn format_location(tab: Option<&TabInfo>) -> String {
    let Some(tab) = tab else {
        return "[No Tab]".to_string();
    };
    let loading = if tab.is_loading { " [loading...]" } else { "" };
    match (tab.title.is_empty(), tab.url.is_empty()) {
        (false, false) => format!("{} — {}{loading}", tab.title, tab.url),
        (false, true) => format!("{}{loading}", tab.title),
        (true, false) => format!("{}{loading}", tab.url),
        (true, true) => format!("about:blank{loading}"),
    }
}
