use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use crate::core::{DebuggerEvent, StepAction, format_scope_bindings};
use crate::engine::rdp_client::{DebuggerCommand, RdpDebuggerClient};
use crate::ui::debugger_layout::{append_row, build_debugger_widgets, clear_list};
use crate::ui::debugger_state::DebuggerState;

const POLL_MS: Duration = Duration::from_millis(50);

/// Built-in GTK4 JavaScript Debugger tab backed by Servo's SpiderMonkey Debugger (`debugger.js`).
#[derive(Clone)]
pub struct DebuggerView {
    root: gtk::Box,
    status_lbl: gtk::Label,
    bp_entry: gtk::Entry,
    sources_list: gtk::ListBox,
    source_buf: gtk::TextBuffer,
    frames_list: gtk::ListBox,
    scopes_buf: gtk::TextBuffer,
    state: Rc<RefCell<DebuggerState>>,
    polling: Rc<Cell<bool>>,
}

impl DebuggerView {
    pub fn new() -> Self {
        let w = build_debugger_widgets();
        let view = Self {
            root: w.root,
            status_lbl: w.status_lbl,
            bp_entry: w.bp_entry,
            sources_list: w.sources_list,
            source_buf: w.source_buf,
            frames_list: w.frames_list,
            scopes_buf: w.scopes_buf,
            state: Rc::new(RefCell::new(DebuggerState::default())),
            polling: Rc::new(Cell::new(false)),
        };
        view.wire(w.action_btns, w.bp_btn);
        view
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.root
    }

    /// Attach to `tab_id`'s Servo DevTools RDP server (if not already attached).
    pub fn connect_to_tab(&self, tab_id: u64, endpoint: Option<(u16, String)>) {
        let Some((port, token)) = endpoint else {
            self.disconnect();
            self.set_status("Waiting for Servo DevTools…", false);
            return;
        };
        {
            let st = self.state.borrow();
            if st.connected_tab == Some(tab_id)
                && st.connected_port == Some(port)
                && st.client.is_some()
            {
                st.send(DebuggerCommand::RefreshSources);
                return;
            }
        }
        self.reset_ui();
        let client = RdpDebuggerClient::connect(port, token);
        *self.state.borrow_mut() = DebuggerState {
            connected_tab: Some(tab_id),
            connected_port: Some(port),
            client: Some(client),
            ..Default::default()
        };
        self.set_status(&format!("Connecting (127.0.0.1:{port})…"), false);
        self.ensure_polling();
    }

    pub fn disconnect(&self) {
        let mut st = self.state.borrow_mut();
        (st.client, st.connected_tab, st.connected_port) = (None, None, None);
        drop(st);
        self.set_status("Disconnected", false);
    }

    fn wire(&self, action_btns: [gtk::Button; 6], bp_btn: gtk::Button) {
        let cmds = [
            DebuggerCommand::Pause,
            DebuggerCommand::Resume(StepAction::Continue),
            DebuggerCommand::Resume(StepAction::StepOver),
            DebuggerCommand::Resume(StepAction::StepIn),
            DebuggerCommand::Resume(StepAction::StepOut),
            DebuggerCommand::RefreshSources,
        ];
        for (btn, cmd) in action_btns.into_iter().zip(cmds) {
            let st = self.state.clone();
            btn.connect_clicked(move |_| st.borrow().send(cmd.clone()));
        }

        let this = self.clone();
        bp_btn.connect_clicked(move |_| this.toggle_breakpoint_from_entry());
        let this = self.clone();
        self.bp_entry
            .connect_activate(move |_| this.toggle_breakpoint_from_entry());

        let bp_entry = self.bp_entry.clone();
        self.source_buf.connect_cursor_position_notify(move |buf| {
            let iter = buf.iter_at_offset(buf.cursor_position());
            bp_entry.set_text(&(iter.line() + 1).max(1).to_string());
        });

        let this = self.clone();
        self.sources_list.connect_row_selected(move |_, row| {
            if let Some(row) = row
                && let Ok(idx) = usize::try_from(row.index())
            {
                let actor = {
                    let mut st = this.state.borrow_mut();
                    let Some(src) = st.sources.get(idx) else {
                        return;
                    };
                    let a = src.actor.clone();
                    st.selected_actor = Some(a.clone());
                    a
                };
                this.state.borrow().send(DebuggerCommand::LoadSource(actor));
                this.render_selected_source();
            }
        });

        let this = self.clone();
        self.frames_list.connect_row_selected(move |_, row| {
            if let Some(row) = row
                && let Ok(idx) = usize::try_from(row.index())
            {
                let actor = this.state.borrow().frames.get(idx).map(|f| f.actor.clone());
                if let Some(a) = actor {
                    this.state.borrow().send(DebuggerCommand::LoadScope(a));
                }
            }
        });
    }

    fn ensure_polling(&self) {
        if self.polling.replace(true) {
            return;
        }
        let this = self.clone();
        glib::timeout_add_local(POLL_MS, move || {
            let mut events = Vec::new();
            let has_client = {
                let st = this.state.borrow();
                if let Some(c) = st.client.as_ref() {
                    while let Some(ev) = c.try_recv() {
                        events.push(ev);
                    }
                    true
                } else {
                    false
                }
            };
            for ev in events {
                this.handle_event(ev);
            }
            if has_client {
                glib::ControlFlow::Continue
            } else {
                this.polling.set(false);
                glib::ControlFlow::Break
            }
        });
    }

    fn handle_event(&self, event: DebuggerEvent) {
        match event {
            DebuggerEvent::Connected { port } => {
                self.set_status(&format!("Running (:{port})"), false)
            }
            DebuggerEvent::Sources(sources) => {
                self.state.borrow_mut().sources = sources;
                clear_list(&self.sources_list);
                for src in &self.state.borrow().sources {
                    append_row(&self.sources_list, &src.display_label());
                }
            }
            DebuggerEvent::SourceContent { actor, content } => {
                self.state
                    .borrow_mut()
                    .source_text
                    .insert(actor.clone(), content);
                if self.state.borrow().selected_actor.as_deref() == Some(actor.as_str()) {
                    self.render_selected_source();
                }
            }
            DebuggerEvent::BreakableLines { actor, lines } => {
                self.state
                    .borrow_mut()
                    .breakable
                    .insert(actor.clone(), lines.into_iter().collect());
                if self.state.borrow().selected_actor.as_deref() == Some(actor.as_str()) {
                    self.render_selected_source();
                }
            }
            DebuggerEvent::Paused {
                reason,
                source_actor,
                line,
            } => {
                self.set_status(&format!("Paused ({reason})"), true);
                {
                    let mut st = self.state.borrow_mut();
                    st.paused_actor = source_actor.clone();
                    st.paused_line = line;
                    if let Some(a) = source_actor {
                        st.selected_actor = Some(a.clone());
                        if !st.source_text.contains_key(&a) {
                            st.send(DebuggerCommand::LoadSource(a));
                        }
                    }
                }
                self.render_selected_source();
            }
            DebuggerEvent::Frames(frames) => {
                self.state.borrow_mut().frames = frames;
                clear_list(&self.frames_list);
                for frame in &self.state.borrow().frames {
                    append_row(&self.frames_list, &frame.display_line());
                }
            }
            DebuggerEvent::ScopeBindings { bindings, .. } => {
                self.scopes_buf.set_text(&format_scope_bindings(&bindings))
            }
            DebuggerEvent::Resumed => {
                let port = self.state.borrow().connected_port.unwrap_or(0);
                self.set_status(&format!("Running (:{port})"), false);
                {
                    let mut st = self.state.borrow_mut();
                    st.paused_actor = None;
                    st.paused_line = None;
                    st.frames.clear();
                }
                clear_list(&self.frames_list);
                self.scopes_buf.set_text("Execution running.");
                self.render_selected_source();
            }
            DebuggerEvent::Disconnected(msg) => {
                self.state.borrow_mut().client = None;
                self.set_status(&msg, false);
            }
        }
    }

    fn toggle_breakpoint_from_entry(&self) {
        let Ok(line) = self.bp_entry.text().trim().parse::<u32>() else {
            return;
        };
        let Some((url, enable)) = self.state.borrow_mut().toggle_breakpoint(line) else {
            return;
        };
        self.state.borrow().send(DebuggerCommand::ToggleBreakpoint {
            source_url: url,
            line,
            column: 0,
            enable,
        });
        self.render_selected_source();
    }

    fn render_selected_source(&self) {
        if let Some(text) = self.state.borrow().rendered_selected_source() {
            self.source_buf.set_text(&text);
        }
    }

    fn reset_ui(&self) {
        clear_list(&self.sources_list);
        clear_list(&self.frames_list);
        self.source_buf
            .set_text("Select a script on the left to inspect or set breakpoints.");
        self.scopes_buf
            .set_text("Pause execution or hit a breakpoint to inspect local scopes.");
    }
    fn set_status(&self, text: &str, paused: bool) {
        self.status_lbl.set_text(text);
        if paused {
            self.status_lbl.add_css_class("debugger-paused");
        } else {
            self.status_lbl.remove_css_class("debugger-paused");
        }
    }
}
