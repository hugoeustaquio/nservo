use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use crate::core::ConsoleLevel;

/// Callbacks emitted by the built-in GTK4 Web Inspector panel.
#[derive(Clone)]
pub struct InspectorCallbacks {
    pub on_eval_js: Rc<dyn Fn(String)>,
    pub on_refresh_dom: Rc<dyn Fn()>,
    pub on_open_debugger: Rc<dyn Fn()>,
    pub on_select_dom_node: Rc<dyn Fn(usize)>,
    pub on_toggle_picker: Rc<dyn Fn()>,
    pub on_clear_console: Rc<dyn Fn()>,
    pub on_close: Rc<dyn Fn()>,
}

pub(super) struct InspectorHeader {
    pub(super) bar: gtk::Box,
    pub(super) find_btn: gtk::Button,
    pub(super) console_tab_btn: gtk::Button,
    pub(super) dom_tab_btn: gtk::Button,
    pub(super) debugger_tab_btn: gtk::Button,
    pub(super) refresh_btn: gtk::Button,
    pub(super) clear_btn: gtk::Button,
    pub(super) close_btn: gtk::Button,
}

/// Build the top inspector header toolbar with mode tabs and action buttons.
pub(super) fn build_inspector_header() -> InspectorHeader {
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    bar.add_css_class("inspector-header");

    let find_btn = make_header_btn("Find");
    find_btn.set_tooltip_text(Some("Pick an element on the page to inspect in DOM view"));
    let console_tab_btn = make_header_btn("Console");
    console_tab_btn.add_css_class("inspector-tab-active");
    let dom_tab_btn = make_header_btn("DOM");
    let debugger_tab_btn = make_header_btn("Debugger");

    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);

    let refresh_btn = make_header_btn("Refresh DOM");
    let clear_btn = make_header_btn("Clear");
    let close_btn = make_header_btn("×");

    for w in [
        find_btn.upcast_ref::<gtk::Widget>(),
        console_tab_btn.upcast_ref(),
        dom_tab_btn.upcast_ref(),
        debugger_tab_btn.upcast_ref(),
        spacer.upcast_ref(),
        refresh_btn.upcast_ref(),
        clear_btn.upcast_ref(),
        close_btn.upcast_ref(),
    ] {
        bar.append(w);
    }

    InspectorHeader {
        bar,
        find_btn,
        console_tab_btn,
        dom_tab_btn,
        debugger_tab_btn,
        refresh_btn,
        clear_btn,
        close_btn,
    }
}

fn make_header_btn(label: &str) -> gtk::Button {
    let btn = gtk::Button::with_label(label);
    btn.add_css_class("inspector-btn");
    btn
}

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
pub(super) fn build_dom_page() -> (
    gtk::Paned,
    gtk::ScrolledWindow,
    gtk::ListBox,
    gtk::TextBuffer,
) {
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
    (paned, left_scroll, list, details_buf)
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
