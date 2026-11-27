use gtk::prelude::*;

/// Bottom Neovim-style prompt bar (`:` for Command mode, `/` for Search mode).
#[derive(Clone)]
pub struct CommandBar {
    container: gtk::Box,
    prefix: gtk::Label,
    entry: gtk::Entry,
}

impl CommandBar {
    pub fn new(on_submit: impl Fn(String) + 'static) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        container.add_css_class("command-bar");
        container.set_hexpand(true);
        container.set_visible(false);

        let prefix = gtk::Label::new(Some(":"));
        prefix.add_css_class("command-prefix");

        let entry = gtk::Entry::new();
        entry.add_css_class("command-entry");
        entry.set_hexpand(true);
        entry.set_has_frame(false);

        entry.connect_activate(move |entry| {
            let text = entry.text().to_string();
            entry.set_text("");
            on_submit(text);
        });

        container.append(&prefix);
        container.append(&entry);

        Self {
            container,
            prefix,
            entry,
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }

    /// Show the `:` command bar and focus it for typing.
    pub fn enter_command_mode(&self) {
        self.enter_command_mode_with("");
    }

    /// Show the `:` command bar pre-filled with `initial` and place the cursor at the end.
    pub fn enter_command_mode_with(&self, initial: &str) {
        self.prefix.set_text(":");
        self.prefix.remove_css_class("search-prefix");
        self.entry.set_text(initial);
        self.container.set_visible(true);
        self.entry.grab_focus();
        self.entry.set_position(-1);
    }

    /// Show the `/` search bar and focus it for typing.
    pub fn enter_search_mode(&self) {
        self.prefix.set_text("/");
        self.prefix.add_css_class("search-prefix");
        self.entry.set_text("");
        self.container.set_visible(true);
        self.entry.grab_focus();
        self.entry.set_position(-1);
    }

    /// Hide the command bar when leaving Command mode.
    pub fn exit_command_mode(&self) {
        self.entry.set_text("");
        self.container.set_visible(false);
    }

    /// True when the command entry is currently empty (used for Neovim Backspace cancel).
    pub fn is_entry_empty(&self) -> bool {
        self.entry.text().is_empty()
    }
}
