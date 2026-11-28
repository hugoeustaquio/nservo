use gtk::prelude::*;

pub(super) struct DebuggerWidgets {
    pub(super) root: gtk::Box,
    pub(super) status_lbl: gtk::Label,
    pub(super) bp_entry: gtk::Entry,
    pub(super) bp_btn: gtk::Button,
    pub(super) action_btns: [gtk::Button; 6],
    pub(super) sources_list: gtk::ListBox,
    pub(super) source_buf: gtk::TextBuffer,
    pub(super) frames_list: gtk::ListBox,
    pub(super) scopes_buf: gtk::TextBuffer,
}

pub(super) fn build_debugger_widgets() -> DebuggerWidgets {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    toolbar.add_css_class("debugger-toolbar");

    let status_lbl = gtk::Label::new(Some("Disconnected"));
    status_lbl.add_css_class("debugger-status");
    let action_btns = ["Pause", "Continue", "Over", "In", "Out", "Refresh"].map(make_btn);
    let bp_entry = gtk::Entry::new();
    bp_entry.set_width_chars(6);
    bp_entry.set_placeholder_text(Some("Line"));
    bp_entry.add_css_class("command-entry");
    let bp_btn = make_btn("Toggle BP");

    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    for b in &action_btns[..5] {
        toolbar.append(b);
    }
    toolbar.append(&bp_entry);
    toolbar.append(&bp_btn);
    toolbar.append(&action_btns[5]);
    toolbar.append(&spacer);
    toolbar.append(&status_lbl);

    let sources_list = gtk::ListBox::new();
    sources_list.add_css_class("inspector-dom-list");
    let source_buf = gtk::TextBuffer::new(None);
    source_buf.set_text("Select a script on the left to inspect or set breakpoints.");
    let frames_list = gtk::ListBox::new();
    frames_list.add_css_class("inspector-dom-list");
    let scopes_buf = gtk::TextBuffer::new(None);
    scopes_buf.set_text("Pause execution or hit a breakpoint to inspect local scopes.");

    let right_paned = gtk::Paned::new(gtk::Orientation::Vertical);
    right_paned.set_start_child(Some(&scroll_wrap(&frames_list)));
    right_paned.set_end_child(Some(&scroll_wrap(&mono_text_view(&scopes_buf))));
    right_paned.set_position(100);

    let inner_paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    inner_paned.set_start_child(Some(&scroll_wrap(&mono_text_view(&source_buf))));
    inner_paned.set_end_child(Some(&right_paned));
    inner_paned.set_position(520);

    let outer_paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    outer_paned.set_start_child(Some(&scroll_wrap(&sources_list)));
    outer_paned.set_end_child(Some(&inner_paned));
    outer_paned.set_position(220);
    outer_paned.set_vexpand(true);
    outer_paned.set_hexpand(true);

    root.append(&toolbar);
    root.append(&outer_paned);

    DebuggerWidgets {
        root,
        status_lbl,
        bp_entry,
        bp_btn,
        action_btns,
        sources_list,
        source_buf,
        frames_list,
        scopes_buf,
    }
}

fn make_btn(label: &str) -> gtk::Button {
    let b = gtk::Button::with_label(label);
    b.add_css_class("inspector-btn");
    b
}

fn scroll_wrap(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        .child(child)
        .vexpand(true)
        .hexpand(true)
        .build()
}

fn mono_text_view(buf: &gtk::TextBuffer) -> gtk::TextView {
    let v = gtk::TextView::with_buffer(buf);
    v.set_editable(false);
    v.set_monospace(true);
    v.add_css_class("inspector-text");
    v
}

pub(super) fn clear_list(list: &gtk::ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

pub(super) fn append_row(list: &gtk::ListBox, text: &str) {
    let lbl = gtk::Label::new(Some(text));
    lbl.set_halign(gtk::Align::Start);
    lbl.set_xalign(0.0);
    lbl.add_css_class("inspector-dom-row");
    list.append(&lbl);
}
