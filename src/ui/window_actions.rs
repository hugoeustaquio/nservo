use gtk::prelude::*;

use crate::core::{
    BookmarkStore, BrowserAction, CompletionSources, HintEvent, HistoryStore, Mode, SearchEvent,
    SessionStore, ZOOM_STEP, step_zoom,
};
use crate::engine::{new_tab_page, scroll, start_page};
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
                if self.inspector.is_picker_active() {
                    self.stop_element_picker();
                } else if self.search.clear(&self.tab_view) {
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
            BrowserAction::OpenStartPageInNewTab => {
                let html = new_tab_page::load_start_page_html();
                self.tab_view
                    .open_html_in_new_tab(new_tab_page::START_PAGE_URL, "New Tab", &html);
            }
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
            BrowserAction::AddAdblockList(url) => {
                self.adblock.add_list(&url, &self.tab_view, &self.status_bar);
            }
            BrowserAction::UpdateAdblockLists => {
                self.adblock.update_lists(&self.tab_view, &self.status_bar);
            }
            BrowserAction::Quit => self.window.close(),
        }
    }

    pub(super) fn load_completion_sources(&self) -> CompletionSources {
        CompletionSources {
            bookmarks: BookmarkStore::default_store()
                .list_recent(100)
                .unwrap_or_default(),
            history: HistoryStore::default_store()
                .recent(200)
                .unwrap_or_default(),
            sessions: SessionStore::default_store()
                .list_names()
                .unwrap_or_default(),
        }
    }

    pub(super) fn handle_navigation(&self, tab_id: u64) {
        if self.mode.get() == Mode::Insert && self.tab_view.active_tab_id() == Some(tab_id) {
            self.set_mode(Mode::Normal);
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
