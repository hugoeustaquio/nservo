use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;

use crate::core::{ConsoleEntry, DomNodeSummary, ElementDetails};
use crate::ui::debugger_view::DebuggerView;
use crate::ui::inspector_views::{attach_repl_history, build_console_page, build_dom_page};

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

/// Lightweight built-in GTK4 Inspector drawer (JS Console/REPL, DOM Inspector & JS Debugger).
#[derive(Clone)]
pub struct InspectorPanel {
    root: gtk::Box,
    stack: gtk::Stack,
    find_btn: gtk::Button,
    console_tab_btn: gtk::Button,
    dom_tab_btn: gtk::Button,
    debugger_tab_btn: gtk::Button,
    console_view: gtk::TextView,
    console_buffer: gtk::TextBuffer,
    repl_entry: gtk::Entry,
    dom_scroll: gtk::ScrolledWindow,
    dom_list: gtk::ListBox,
    dom_details: gtk::TextBuffer,
    dom_indices: Rc<RefCell<Vec<usize>>>,
    debugger: DebuggerView,
    picker_active: Rc<Cell<bool>>,
    history: Rc<RefCell<Vec<String>>>,
    history_pos: Rc<Cell<Option<usize>>>,
}

impl InspectorPanel {
    pub fn new(callbacks: InspectorCallbacks) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("inspector-panel");
        root.set_size_request(-1, 240);
        root.set_visible(false);

        let header = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        header.add_css_class("inspector-header");

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
            header.append(w);
        }

        let (console_page, console_view, console_buffer, repl_entry) = build_console_page();
        let (dom_page, dom_scroll, dom_list, dom_details) = build_dom_page();
        let debugger = DebuggerView::new();

        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        stack.set_hexpand(true);
        stack.add_named(&console_page, Some("console"));
        stack.add_named(&dom_page, Some("dom"));
        stack.add_named(debugger.widget(), Some("debugger"));
        stack.set_visible_child_name("console");

        root.append(&header);
        root.append(&stack);

        let panel = Self {
            root, stack, find_btn, console_tab_btn, dom_tab_btn, debugger_tab_btn,
            console_view, console_buffer, repl_entry, dom_scroll, dom_list, dom_details,
            dom_indices: Rc::new(RefCell::new(Vec::new())),
            debugger,
            picker_active: Rc::new(Cell::new(false)),
            history: Rc::new(RefCell::new(Vec::new())),
            history_pos: Rc::new(Cell::new(None)),
        };

        panel.wire_controls(refresh_btn, clear_btn, close_btn, callbacks);
        panel
    }

    pub fn widget(&self) -> &gtk::Box { &self.root }
    pub fn debugger(&self) -> &DebuggerView { &self.debugger }
    pub fn is_open(&self) -> bool { self.root.is_visible() }
    pub fn is_dom_tab_active(&self) -> bool { self.stack.visible_child_name().as_deref() == Some("dom") }
    pub fn is_debugger_tab_active(&self) -> bool { self.stack.visible_child_name().as_deref() == Some("debugger") }

    pub fn show_tab(&self, name: &str) {
        self.stack.set_visible_child_name(name);
        for (btn, target) in [
            (&self.console_tab_btn, "console"),
            (&self.dom_tab_btn, "dom"),
            (&self.debugger_tab_btn, "debugger"),
        ] {
            if target == name { btn.add_css_class("inspector-tab-active"); }
            else { btn.remove_css_class("inspector-tab-active"); }
        }
    }

    pub fn show_dom_tab(&self) { self.show_tab("dom"); }
    pub fn is_picker_active(&self) -> bool { self.picker_active.get() }

    pub fn set_picker_active(&self, active: bool) {
        self.picker_active.set(active);
        if active { self.find_btn.add_css_class("inspector-tab-active"); }
        else { self.find_btn.remove_css_class("inspector-tab-active"); }
    }

    pub fn set_open(&self, open: bool) {
        self.root.set_visible(open);
        if open && self.stack.visible_child_name().as_deref() == Some("console") {
            self.repl_entry.grab_focus();
        }
    }

    /// Returns `true` when keyboard focus is inside the inspector drawer.
    pub fn has_focus_within(&self) -> bool {
        if !self.root.is_visible() { return false; }
        self.root.root().and_then(|r| r.focus()).is_some_and(|w| {
            w == *self.root.upcast_ref::<gtk::Widget>() || w.is_ancestor(&self.root)
        })
    }

    pub fn render_console(&self, entries: &[ConsoleEntry]) {
        self.console_buffer.set_text("");
        for entry in entries { self.append_console_entry(entry); }
    }

    pub fn append_console_entry(&self, entry: &ConsoleEntry) {
        let mut end = self.console_buffer.end_iter();
        let line = format!("[{}] {}\n", entry.level.prefix(), entry.text);
        self.console_buffer.insert_with_tags_by_name(&mut end, &line, &[entry.level.tag_name()]);
        let mut end_after = self.console_buffer.end_iter();
        self.console_view.scroll_to_iter(&mut end_after, 0.0, false, 0.0, 1.0);
    }

    pub fn set_dom_nodes(&self, nodes: &[DomNodeSummary]) {
        while let Some(child) = self.dom_list.first_child() { self.dom_list.remove(&child); }
        let mut indices = self.dom_indices.borrow_mut();
        indices.clear();
        for node in nodes {
            indices.push(node.index);
            let label = gtk::Label::new(Some(&node.display_line()));
            label.set_halign(gtk::Align::Start);
            label.set_xalign(0.0);
            label.add_css_class("inspector-dom-row");
            self.dom_list.append(&label);
        }
    }

    pub fn select_dom_node(&self, node_index: usize) {
        let pos = self.dom_indices.borrow().iter().position(|&idx| idx == node_index);
        let Some(row_idx) = pos.and_then(|p| i32::try_from(p).ok()) else { return };
        let Some(row) = self.dom_list.row_at_index(row_idx) else { return };
        self.dom_list.select_row(Some(&row));
        let (list, scroll) = (self.dom_list.clone(), self.dom_scroll.clone());
        glib::idle_add_local_once(move || {
            if let Some(bounds) = row.compute_bounds(&list) {
                let adj = scroll.vadjustment();
                let target = f64::from(bounds.y()) - (adj.page_size() - f64::from(bounds.height())) / 2.0;
                adj.set_value(target.clamp(adj.lower(), (adj.upper() - adj.page_size()).max(0.0)));
            }
        });
    }

    pub fn set_element_details(&self, details: &ElementDetails) {
        self.dom_details.set_text(&details.format_text());
    }

    pub fn clear_dom(&self) {
        while let Some(child) = self.dom_list.first_child() { self.dom_list.remove(&child); }
        self.dom_indices.borrow_mut().clear();
        self.dom_details.set_text("Select an element on the left to inspect its attributes and styles.");
    }

    fn wire_controls(
        &self,
        refresh_btn: gtk::Button,
        clear_btn: gtk::Button,
        close_btn: gtk::Button,
        callbacks: InspectorCallbacks,
    ) {
        let this = self.clone();
        self.console_tab_btn.connect_clicked(move |_| {
            this.show_tab("console");
            this.repl_entry.grab_focus();
        });

        let (this, on_refresh) = (self.clone(), callbacks.on_refresh_dom.clone());
        self.dom_tab_btn.connect_clicked(move |_| {
            this.show_tab("dom");
            on_refresh();
        });

        let (this, on_dbg) = (self.clone(), callbacks.on_open_debugger.clone());
        self.debugger_tab_btn.connect_clicked(move |_| {
            this.show_tab("debugger");
            on_dbg();
        });

        let on_toggle_picker = callbacks.on_toggle_picker.clone();
        self.find_btn.connect_clicked(move |_| on_toggle_picker());

        let on_refresh = callbacks.on_refresh_dom.clone();
        refresh_btn.connect_clicked(move |_| on_refresh());

        let on_clear = callbacks.on_clear_console.clone();
        clear_btn.connect_clicked(move |_| on_clear());

        let on_close = callbacks.on_close.clone();
        close_btn.connect_clicked(move |_| on_close());

        let (on_eval, history, pos) = (callbacks.on_eval_js, self.history.clone(), self.history_pos.clone());
        self.repl_entry.connect_activate(move |entry| {
            let expr = entry.text().trim().to_string();
            entry.set_text("");
            pos.set(None);
            if expr.is_empty() { return; }
            history.borrow_mut().push(expr.clone());
            on_eval(expr);
        });

        attach_repl_history(&self.repl_entry, self.history.clone(), self.history_pos.clone());

        let (indices, on_select) = (self.dom_indices.clone(), callbacks.on_select_dom_node);
        self.dom_list.connect_row_selected(move |_, row| {
            if let Some(row) = row
                && let Ok(pos) = usize::try_from(row.index())
                && let Some(&node_index) = indices.borrow().get(pos)
            {
                on_select(node_index);
            }
        });
    }
}

fn make_header_btn(label: &str) -> gtk::Button {
    let btn = gtk::Button::with_label(label);
    btn.add_css_class("inspector-btn");
    btn
}
