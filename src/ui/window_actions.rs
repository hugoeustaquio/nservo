use gtk::prelude::*;

use crate::core::{
    BrowserAction, ConsoleEntry, ConsoleLevel, HintEvent, InspectorEvent, Mode, SearchEvent,
    ZOOM_STEP, step_zoom,
};
use crate::engine::{scroll, start_page};
use crate::ui::window::BrowserWindow;

impl BrowserWindow {
    pub(super) fn handle_action(&self, action: BrowserAction) {
        if let Some(view) = self.tab_view.active_web_view()
            && scroll::try_apply(&view, &action)
        {
            return;
        }
        match action {
            BrowserAction::EnterMode(mode) => self.set_mode(mode),
            BrowserAction::EnterCommandWith(prefix) => self.transition_mode(Mode::Command, &prefix),
            BrowserAction::EditCurrentUrl => {
                let prefix = match self.tab_view.active_url() {
                    Some(url) if !url.is_empty() => format!("open {url}"),
                    _ => "open ".to_string(),
                };
                self.transition_mode(Mode::Command, &prefix);
            }
            BrowserAction::StartHints { new_tab } => {
                self.set_mode(Mode::Hint);
                self.hints.start(&self.tab_view, new_tab);
            }
            BrowserAction::HintChar(ch) => {
                let typed = self.hints.push_char(&self.tab_view, ch);
                self.set_pending_display(Some(typed));
            }
            BrowserAction::HintBackspace => match self.hints.pop_char(&self.tab_view) {
                Some(rem) => self.set_pending_display((!rem.is_empty()).then_some(rem)),
                None => self.set_mode(Mode::Normal),
            },
            BrowserAction::SearchNext => {
                self.search.step(&self.tab_view, &self.status_bar, 1);
            }
            BrowserAction::SearchPrev => {
                self.search.step(&self.tab_view, &self.status_bar, -1);
            }
            BrowserAction::ClearSearch => {
                if self.search.clear(&self.tab_view) {
                    self.sync_chrome();
                }
            }
            BrowserAction::InsertText(text) => {
                if let Some(tab) = self.tab_view.active_tab() {
                    tab.insert_utf16_text(&text);
                }
            }
            BrowserAction::MoveWord { direction, select } => {
                if let Some(tab) = self.tab_view.active_tab() {
                    tab.move_word(direction, select);
                }
            }
            BrowserAction::MoveLineEdge(edge) => {
                if let Some(tab) = self.tab_view.active_tab() {
                    tab.move_line_edge(edge);
                }
            }
            BrowserAction::CopySelection => {
                if let Some(tab) = self.tab_view.active_tab() {
                    tab.copy_selection();
                }
            }
            BrowserAction::CutSelection => {
                if let Some(tab) = self.tab_view.active_tab() {
                    tab.cut_selection();
                }
            }
            BrowserAction::PasteClipboard => {
                if let Some(tab) = self.tab_view.active_tab() {
                    tab.paste_from_clipboard();
                }
            }
            BrowserAction::ScrollDown(_)
            | BrowserAction::ScrollUp(_)
            | BrowserAction::ScrollLeft(_)
            | BrowserAction::ScrollRight(_)
            | BrowserAction::ScrollTop
            | BrowserAction::ScrollBottom => {}
            BrowserAction::NextTab => self.tab_view.next_tab(),
            BrowserAction::PrevTab => self.tab_view.prev_tab(),
            BrowserAction::OpenInCurrentTab(url) => self.tab_view.load_url_in_active_tab(&url),
            BrowserAction::OpenInNewTab(url) => self.tab_view.open_url_in_new_tab(&url),
            BrowserAction::CloseTab if !self.tab_view.close_active_tab() => self.window.close(),
            BrowserAction::CloseTab => {}
            BrowserAction::ReopenClosedTab => {
                if !self.tab_view.reopen_closed_tab() {
                    self.status_bar.show_notice("No closed tabs to reopen");
                }
            }
            BrowserAction::Reload => self.tab_view.reload_active_tab(),
            BrowserAction::GoBack => self.with_active_view(|v| v.go_back()),
            BrowserAction::GoForward => self.with_active_view(|v| v.go_forward()),
            BrowserAction::SetZoom(level) => self.with_active_view(|v| v.set_zoom_level(level)),
            BrowserAction::ZoomIn => {
                self.with_active_view(|v| v.set_zoom_level(step_zoom(v.zoom_level(), ZOOM_STEP)));
            }
            BrowserAction::ZoomOut => {
                self.with_active_view(|v| v.set_zoom_level(step_zoom(v.zoom_level(), -ZOOM_STEP)));
            }
            BrowserAction::ShowHelp => {
                self.tab_view.load_html_in_active_tab(
                    "about:help",
                    "nservo — Help",
                    start_page::HELP_PAGE_HTML,
                );
            }
            BrowserAction::ToggleInspector => {
                self.set_inspector_open(!self.inspector.is_open());
            }
            BrowserAction::SaveSession(name) => {
                self.sessions.save(&self.tab_view, &self.status_bar, &name);
            }
            BrowserAction::SaveSessionAndQuit(name) => {
                if self.sessions.save(&self.tab_view, &self.status_bar, &name) {
                    self.window.close();
                }
            }
            BrowserAction::LoadSession(name) => {
                self.sessions.load(&self.tab_view, &self.status_bar, &name);
            }
            BrowserAction::AddBookmark => {
                self.bookmarks.add_current(&self.tab_view, &self.status_bar);
            }
            BrowserAction::ShowBookmarks => {
                self.bookmarks.show(&self.tab_view, &self.status_bar);
            }
            BrowserAction::RemoveBookmark(target) => {
                self.bookmarks
                    .remove(&self.tab_view, &self.status_bar, target.as_deref());
            }
            BrowserAction::ShowHistory(query) => {
                self.history
                    .show(&self.tab_view, &self.status_bar, query.as_deref());
            }
            BrowserAction::ClearHistory => {
                self.history.clear(&self.tab_view, &self.status_bar);
            }
            BrowserAction::Quit => self.window.close(),
        }
    }

    pub(super) fn handle_navigation(&self, tab_id: u64) {
        if self.mode.get() == Mode::Insert && self.tab_view.active_tab_id() == Some(tab_id) {
            self.set_mode(Mode::Normal);
        }
    }

    pub(super) fn set_inspector_open(&self, open: bool) {
        self.inspector.set_open(open);
        if open {
            if let Some(tab_id) = self.tab_view.active_tab_id() {
                self.last_inspected_tab.set(Some(tab_id));
                let entries = self.console_store.borrow().entries(tab_id);
                self.inspector.render_console(&entries);
            }
            self.refresh_inspector_dom();
        } else {
            if let Some(tab) = self.tab_view.active_tab() {
                tab.clear_inspector_highlight();
            }
            self.tab_view.focus_active();
        }
    }

    pub(super) fn sync_inspector_for_active_tab(&self, active_id: Option<u64>) {
        if self.last_inspected_tab.get() == active_id {
            return;
        }
        self.last_inspected_tab.set(active_id);
        if !self.inspector.is_open() {
            return;
        }
        self.inspector.clear_dom();
        if let Some(tab_id) = active_id {
            let entries = self.console_store.borrow().entries(tab_id);
            self.inspector.render_console(&entries);
            if self.inspector.is_dom_tab_active() {
                self.refresh_inspector_dom();
            }
        }
    }

    pub(super) fn eval_inspector_js(&self, expr: &str) {
        let Some(tab_id) = self.tab_view.active_tab_id() else {
            return;
        };
        let entry = ConsoleEntry {
            level: ConsoleLevel::Input,
            text: expr.to_string(),
        };
        self.console_store.borrow_mut().push(tab_id, entry.clone());
        self.inspector.append_console_entry(&entry);
        if let Some(tab) = self.tab_view.active_tab() {
            tab.eval_inspector_js(expr);
        }
    }

    pub(super) fn refresh_inspector_dom(&self) {
        if let Some(tab) = self.tab_view.active_tab() {
            tab.request_dom_snapshot();
        }
    }

    pub(super) fn inspect_dom_node(&self, index: usize) {
        if let Some(tab) = self.tab_view.active_tab() {
            tab.inspect_dom_node(index);
        }
    }

    pub(super) fn clear_active_console(&self) {
        if let Some(tab_id) = self.tab_view.active_tab_id() {
            self.console_store.borrow_mut().clear(tab_id);
            self.inspector.render_console(&[]);
        }
    }

    pub(super) fn handle_inspector_event(&self, tab_id: u64, event: InspectorEvent) {
        let is_active = self.tab_view.active_tab_id() == Some(tab_id);
        match event {
            InspectorEvent::Console(entry) => {
                self.console_store.borrow_mut().push(tab_id, entry.clone());
                if is_active && self.inspector.is_open() {
                    self.inspector.append_console_entry(&entry);
                }
            }
            InspectorEvent::DomTree(nodes) => {
                if is_active {
                    self.inspector.set_dom_nodes(&nodes);
                }
            }
            InspectorEvent::ElementDetails(details) => {
                if is_active {
                    self.inspector.set_element_details(&details);
                }
            }
        }
    }

    pub(super) fn handle_hint_event(&self, tab_id: u64, event: HintEvent) {
        if self.mode.get() != Mode::Hint || self.tab_view.active_tab_id() != Some(tab_id) {
            return;
        }
        match event {
            HintEvent::Shown { .. } => {}
            HintEvent::Activated { is_input } => {
                self.set_mode(if is_input { Mode::Insert } else { Mode::Normal });
            }
            HintEvent::OpenInNewTab { url } => {
                self.set_mode(Mode::Normal);
                self.tab_view.open_url_in_new_tab(&url);
            }
            HintEvent::Cancelled => self.set_mode(Mode::Normal),
            HintEvent::Empty => {
                self.set_mode(Mode::Normal);
                self.status_bar.show_notice("No clickable elements found");
            }
        }
    }

    pub(super) fn handle_search_event(&self, tab_id: u64, event: SearchEvent) {
        if self.tab_view.active_tab_id() == Some(tab_id) {
            self.search.handle_event(&self.status_bar, event);
        }
    }

    fn with_active_view(&self, f: impl FnOnce(&servo_gtk::WebView)) {
        if let Some(view) = self.tab_view.active_web_view() {
            f(&view);
        }
    }
}
