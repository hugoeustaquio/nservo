use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::gdk;
use gtk::glib;
use gtk::pango;
use gtk::prelude::*;

use crate::core::{CompletionItem, CompletionKind, CompletionSources, complete_command_input};

/// Bottom Neovim-style prompt bar (`:` for Command mode, `/` for Search mode)
/// with an fzf-style fuzzy completion list for commands, URLs, and sessions.
#[derive(Clone)]
pub struct CommandBar {
    root: gtk::Box,
    prefix: gtk::Label,
    entry: gtk::Entry,
    popup_scroll: gtk::ScrolledWindow,
    popup_list: gtk::ListBox,
    command_mode: Rc<Cell<bool>>,
    syncing: Rc<Cell<bool>>,
    sources: Rc<RefCell<CompletionSources>>,
    items: Rc<RefCell<Vec<CompletionItem>>>,
    selected: Rc<Cell<Option<usize>>>,
    load_sources: Rc<dyn Fn() -> CompletionSources>,
}

impl CommandBar {
    pub fn new(
        on_submit: impl Fn(String) + 'static,
        load_sources: impl Fn() -> CompletionSources + 'static,
    ) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.set_hexpand(true);
        root.set_visible(false);

        let popup_list = gtk::ListBox::new();
        popup_list.add_css_class("completion-list");
        popup_list.set_selection_mode(gtk::SelectionMode::Single);
        popup_list.set_activate_on_single_click(true);
        popup_list.set_can_focus(false);

        let popup_scroll = gtk::ScrolledWindow::builder()
            .child(&popup_list)
            .hexpand(true)
            .propagate_natural_height(true)
            .max_content_height(220)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        popup_scroll.add_css_class("completion-scroll");
        popup_scroll.set_visible(false);

        let bar = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        bar.add_css_class("command-bar");
        bar.set_hexpand(true);

        let prefix = gtk::Label::new(Some(":"));
        prefix.add_css_class("command-prefix");

        let entry = gtk::Entry::new();
        entry.add_css_class("command-entry");
        entry.set_hexpand(true);
        entry.set_has_frame(false);

        bar.append(&prefix);
        bar.append(&entry);
        root.append(&popup_scroll);
        root.append(&bar);

        let bar_widget = Self {
            root,
            prefix,
            entry,
            popup_scroll,
            popup_list,
            command_mode: Rc::new(Cell::new(false)),
            syncing: Rc::new(Cell::new(false)),
            sources: Rc::new(RefCell::new(CompletionSources::default())),
            items: Rc::new(RefCell::new(Vec::new())),
            selected: Rc::new(Cell::new(None)),
            load_sources: Rc::new(load_sources),
        };
        bar_widget.wire_events(Rc::new(on_submit));
        bar_widget
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.root
    }

    /// Show the `:` command bar and focus it for typing.
    pub fn enter_command_mode(&self) {
        self.enter_command_mode_with("");
    }

    /// Show the `:` command bar pre-filled with `initial` and place the cursor at the end.
    pub fn enter_command_mode_with(&self, initial: &str) {
        self.command_mode.set(true);
        *self.sources.borrow_mut() = (self.load_sources)();
        self.prefix.set_text(":");
        self.prefix.remove_css_class("search-prefix");
        self.syncing.set(true);
        self.entry.set_text(initial);
        self.syncing.set(false);
        self.root.set_visible(true);
        self.refresh_completions(initial);
        self.entry.grab_focus();
        self.entry.set_position(-1);
    }

    /// Show the `/` search bar and focus it for typing.
    pub fn enter_search_mode(&self) {
        self.command_mode.set(false);
        self.clear_completions();
        self.prefix.set_text("/");
        self.prefix.add_css_class("search-prefix");
        self.syncing.set(true);
        self.entry.set_text("");
        self.syncing.set(false);
        self.root.set_visible(true);
        self.entry.grab_focus();
        self.entry.set_position(-1);
    }

    /// Hide the command bar and any completion popup when leaving Command/Search mode.
    pub fn exit_command_mode(&self) {
        self.command_mode.set(false);
        self.clear_completions();
        self.syncing.set(true);
        self.entry.set_text("");
        self.syncing.set(false);
        self.root.set_visible(false);
    }

    /// True when the command entry is currently empty (used for Neovim Backspace cancel).
    pub fn is_entry_empty(&self) -> bool {
        self.entry.text().is_empty()
    }

    fn wire_events(&self, on_submit: Rc<dyn Fn(String)>) {
        let submit_activate = on_submit.clone();
        self.entry.connect_activate(move |entry| {
            let text = entry.text().to_string();
            entry.set_text("");
            submit_activate(text);
        });

        let this = self.clone();
        self.entry.connect_changed(move |entry| {
            if this.syncing.get() || !this.command_mode.get() {
                return;
            }
            this.refresh_completions(&entry.text());
        });

        let key_ctrl = gtk::EventControllerKey::new();
        key_ctrl.set_propagation_phase(gtk::PropagationPhase::Capture);
        let this = self.clone();
        key_ctrl.connect_key_pressed(move |_, key, _, mods| {
            if !this.command_mode.get() || this.items.borrow().is_empty() {
                return glib::Propagation::Proceed;
            }
            let ctrl = mods.contains(gdk::ModifierType::CONTROL_MASK);
            let step = match key {
                gdk::Key::Tab | gdk::Key::Down => Some(1i32),
                gdk::Key::ISO_Left_Tab | gdk::Key::Up => Some(-1),
                gdk::Key::n | gdk::Key::j if ctrl => Some(1),
                gdk::Key::p | gdk::Key::k if ctrl => Some(-1),
                _ => None,
            };
            if let Some(delta) = step {
                this.step_selection(delta);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        self.entry.add_controller(key_ctrl);

        let this = self.clone();
        self.popup_list.connect_row_activated(move |_, row| {
            let Ok(idx) = usize::try_from(row.index()) else { return };
            let chosen = this.items.borrow().get(idx).map(|i| i.insert_text.clone());
            if let Some(text) = chosen {
                on_submit(text);
            }
        });
    }

    fn refresh_completions(&self, input: &str) {
        let next = complete_command_input(input, &self.sources.borrow());
        self.selected.set(None);
        while let Some(child) = self.popup_list.first_child() {
            self.popup_list.remove(&child);
        }
        for item in &next {
            self.popup_list.append(&build_completion_row(item));
        }
        let has_items = !next.is_empty();
        *self.items.borrow_mut() = next;
        self.popup_scroll.set_visible(has_items);
    }

    fn clear_completions(&self) {
        self.selected.set(None);
        self.items.borrow_mut().clear();
        while let Some(child) = self.popup_list.first_child() {
            self.popup_list.remove(&child);
        }
        self.popup_scroll.set_visible(false);
    }

    fn step_selection(&self, delta: i32) {
        let len = self.items.borrow().len();
        if len == 0 {
            return;
        }
        let next_idx = match self.selected.get() {
            None if delta > 0 => 0,
            None => len - 1,
            Some(cur) => ((cur as isize + delta as isize).rem_euclid(len as isize)) as usize,
        };
        self.selected.set(Some(next_idx));
        if let Some(row) = i32::try_from(next_idx)
            .ok()
            .and_then(|i| self.popup_list.row_at_index(i))
        {
            self.popup_list.select_row(Some(&row));
        }
        let insert = self.items.borrow()[next_idx].insert_text.clone();
        self.syncing.set(true);
        self.entry.set_text(&insert);
        self.entry.set_position(-1);
        self.syncing.set(false);
    }
}

fn build_completion_row(item: &CompletionItem) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let badge = gtk::Label::new(Some(item.kind.badge()));
    badge.add_css_class("completion-badge");
    match item.kind {
        CompletionKind::Bookmark => badge.add_css_class("kind-bm"),
        CompletionKind::History => badge.add_css_class("kind-hist"),
        CompletionKind::Session => badge.add_css_class("kind-sess"),
        CompletionKind::Command => {}
    }

    let primary = gtk::Label::new(Some(&item.primary));
    primary.add_css_class("completion-primary");
    primary.set_xalign(0.0);
    primary.set_ellipsize(pango::EllipsizeMode::End);

    row.append(&badge);
    row.append(&primary);

    if !item.secondary.is_empty() {
        let secondary = gtk::Label::new(Some(&item.secondary));
        secondary.add_css_class("completion-secondary");
        secondary.set_xalign(0.0);
        secondary.set_hexpand(true);
        secondary.set_ellipsize(pango::EllipsizeMode::End);
        row.append(&secondary);
    }
    row
}
