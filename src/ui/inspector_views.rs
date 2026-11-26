use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use crate::core::ConsoleLevel;

/// Build the Console tab (scrollable log output + JS REPL entry).
pub(super) fn build_console_page() -> (gtk::Box, gtk::TextView, gtk::TextBuffer, gtk::Entry) {
    let buffer = gtk::TextBuffer::new(None);
    for (level, color) in [
        (ConsoleLevel::Log, "#c0caf5"),
        (ConsoleLevel::Info, "#7aa2f7"),
        (ConsoleLevel::Warn, "#e0af68"),
        (ConsoleLevel::Error, "#f7768e"),
        (ConsoleLevel::Debug, "#787c99"),
        (ConsoleLevel::Input, "#bb9af7"),
        (ConsoleLevel::Result, "#9ece6a"),
    ] {
        let tag = gtk::TextTag::builder()
            .name(level.tag_name())
            .foreground(color)
            .build();
        buffer.tag_table().add(&tag);
    }

    let view = gtk::TextView::with_buffer(&buffer);
    view.set_editable(false);
    view.set_cursor_visible(false);
    view.set_monospace(true);
    view.set_wrap_mode(gtk::WrapMode::WordChar);
    view.add_css_class("inspector-text");

    let scroll = gtk::ScrolledWindow::builder()
        .child(&view)
        .vexpand(true)
        .hexpand(true)
        .build();

    let repl_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    repl_bar.add_css_class("inspector-repl");
    let prompt = gtk::Label::new(Some(">"));
    prompt.add_css_class("command-prefix");
    let entry = gtk::Entry::new();
    entry.set_hexpand(true);
    entry.set_placeholder_text(Some("Evaluate JavaScript... (Esc returns to page)"));
    entry.add_css_class("command-entry");
    repl_bar.append(&prompt);
    repl_bar.append(&entry);

    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page.append(&scroll);
    page.append(&repl_bar);
    (page, view, buffer, entry)
}

/// Build the DOM tab (left element list + right attributes/styles pane).
pub(super) fn build_dom_page() -> (gtk::Paned, gtk::ListBox, gtk::TextBuffer) {
    let list = gtk::ListBox::new();
    list.add_css_class("inspector-dom-list");
    let left_scroll = gtk::ScrolledWindow::builder()
        .child(&list)
        .vexpand(true)
        .hexpand(true)
        .build();

    let details_buf = gtk::TextBuffer::new(None);
    details_buf.set_text("Select an element on the left to inspect its attributes and styles.");
    let details_view = gtk::TextView::with_buffer(&details_buf);
    details_view.set_editable(false);
    details_view.set_cursor_visible(false);
    details_view.set_monospace(true);
    details_view.set_wrap_mode(gtk::WrapMode::WordChar);
    details_view.add_css_class("inspector-text");

    let right_scroll = gtk::ScrolledWindow::builder()
        .child(&details_view)
        .vexpand(true)
        .hexpand(true)
        .build();

    let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    paned.set_start_child(Some(&left_scroll));
    paned.set_end_child(Some(&right_scroll));
    paned.set_position(480);
    paned.set_vexpand(true);
    paned.set_hexpand(true);
    (paned, list, details_buf)
}

/// Attach `Up`/`Down` command-history navigation to the REPL `gtk::Entry`.
pub(super) fn attach_repl_history(
    entry: &gtk::Entry,
    history: Rc<RefCell<Vec<String>>>,
    history_pos: Rc<Cell<Option<usize>>>,
) {
    let entry_clone = entry.clone();
    let key_ctrl = gtk::EventControllerKey::new();
    key_ctrl.connect_key_pressed(move |_, key, _, _| {
        let items = history.borrow();
        if items.is_empty() {
            return glib::Propagation::Proceed;
        }
        let next = match key {
            gdk::Key::Up => Some(
                history_pos
                    .get()
                    .map_or(items.len() - 1, |i| i.saturating_sub(1)),
            ),
            gdk::Key::Down => match history_pos.get() {
                Some(i) if i + 1 < items.len() => Some(i + 1),
                Some(_) => {
                    history_pos.set(None);
                    entry_clone.set_text("");
                    return glib::Propagation::Stop;
                }
                None => return glib::Propagation::Proceed,
            },
            _ => return glib::Propagation::Proceed,
        };
        if let Some(idx) = next {
            history_pos.set(Some(idx));
            entry_clone.set_text(&items[idx]);
            entry_clone.set_position(-1);
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    entry.add_controller(key_ctrl);
}
