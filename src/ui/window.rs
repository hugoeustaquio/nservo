use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::prelude::*;

use crate::core::{BrowserAction, HintEvent, Mode, parse_command};
use crate::engine::{scroll, start_page};
use crate::ui::command_bar::CommandBar;
use crate::ui::hint_controller::HintController;
use crate::ui::key_handler::{KeyCallbacks, KeyHandler};
use crate::ui::session_controller::SessionController;
use crate::ui::status_bar::{StatusBar, StatusSnapshot};
use crate::ui::tab_bar::TabBar;
use crate::ui::tab_view::{TabView, TabViewCallbacks};

/// Top-level Neovim-inspired GTK4 browser window.
pub struct BrowserWindow {
    window: gtk::ApplicationWindow,
    tab_bar: TabBar,
    tab_view: TabView,
    status_bar: StatusBar,
    command_bar: CommandBar,
    key_handler: KeyHandler,
    sessions: SessionController,
    hints: HintController,
    mode: Rc<Cell<Mode>>,
    pending_keys: RefCell<Option<String>>,
}

impl BrowserWindow {
    /// Build, wire, and present a new browser window.
    pub fn open(app: &gtk::Application, initial_url: Option<&str>) {
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("nservo")
            .default_width(1200)
            .default_height(800)
            .build();
        window.add_css_class("nservo-window");

        let mode = Rc::new(Cell::new(Mode::Normal));
        let key_handler = KeyHandler::new(mode.clone());
        let status_bar = StatusBar::new();

        // Late-bound controller slot so widget callbacks can reach `Rc<Self>` cleanly.
        let controller_slot: Rc<RefCell<Option<Rc<Self>>>> = Rc::new(RefCell::new(None));

        let tab_bar = {
            let slot = controller_slot.clone();
            TabBar::new(move |index| {
                let ctrl = slot.borrow().clone();
                if let Some(ctrl) = ctrl {
                    ctrl.exit_hint_mode_if_active();
                    ctrl.tab_view.select_index(index);
                }
            })
        };

        let command_bar = {
            let slot = controller_slot.clone();
            CommandBar::new(move |raw| {
                let ctrl = slot.borrow().clone();
                if let Some(ctrl) = ctrl {
                    ctrl.submit_command(&raw);
                }
            })
        };

        let tab_view = {
            let slot_change = controller_slot.clone();
            let slot_popup = controller_slot.clone();
            let slot_hint = controller_slot.clone();
            TabView::new(TabViewCallbacks {
                on_state_changed: Rc::new(move || {
                    let ctrl = slot_change.borrow().clone();
                    if let Some(ctrl) = ctrl {
                        ctrl.sync_chrome();
                    }
                }),
                on_popup_url: Rc::new(move |url| {
                    let ctrl = slot_popup.borrow().clone();
                    if let Some(ctrl) = ctrl {
                        ctrl.tab_view.open_url_in_new_tab(&url);
                    }
                }),
                on_hint_event: Rc::new(move |tab_id, event| {
                    let ctrl = slot_hint.borrow().clone();
                    if let Some(ctrl) = ctrl {
                        ctrl.handle_hint_event(tab_id, event);
                    }
                }),
            })
        };

        let layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        layout.append(tab_bar.widget());
        layout.append(tab_view.widget());
        layout.append(status_bar.widget());
        layout.append(command_bar.widget());
        window.set_child(Some(&layout));

        let controller = Rc::new(Self {
            window: window.clone(),
            tab_bar,
            tab_view,
            status_bar,
            command_bar,
            key_handler,
            sessions: SessionController::new(),
            hints: HintController::new(),
            mode,
            pending_keys: RefCell::new(None),
        });

        *controller_slot.borrow_mut() = Some(controller.clone());
        controller.attach_key_handler();

        let slot_cleanup = controller_slot;
        window.connect_close_request(move |_| {
            slot_cleanup.borrow_mut().take();
            gtk::glib::Propagation::Proceed
        });

        window.present();
        controller
            .sessions
            .open_initial_tabs(&controller.tab_view, initial_url);
    }

    fn attach_key_handler(self: &Rc<Self>) {
        let on_action_ctrl = self.clone();
        let on_pending_ctrl = self.clone();
        let cmd_bar = self.command_bar.clone();

        self.key_handler.attach(
            &self.window,
            KeyCallbacks {
                on_action: Rc::new(move |action| on_action_ctrl.handle_action(action)),
                on_pending_changed: Rc::new(move |pending| {
                    *on_pending_ctrl.pending_keys.borrow_mut() = pending.clone();
                    on_pending_ctrl
                        .status_bar
                        .set_pending_keys(pending.as_deref());
                }),
                is_command_entry_empty: Rc::new(move || cmd_bar.is_entry_empty()),
            },
        );
    }

    fn submit_command(&self, raw: &str) {
        self.set_mode(Mode::Normal);
        match parse_command(raw) {
            Ok(action) => self.handle_action(action),
            Err(err) => self.status_bar.show_error(&err.to_string()),
        }
    }

    fn set_mode(&self, next: Mode) {
        if self.mode.get() == Mode::Hint && next != Mode::Hint {
            self.hints.clear(&self.tab_view);
        }

        self.mode.set(next);
        self.key_handler.reset_pending();
        *self.pending_keys.borrow_mut() = None;

        match next {
            Mode::Normal | Mode::Input | Mode::Hint => {
                self.command_bar.exit_command_mode();
                self.tab_view.focus_active();
            }
            Mode::Command => {
                self.command_bar.enter_command_mode();
            }
        }

        self.sync_chrome();
    }

    fn exit_hint_mode_if_active(&self) {
        if self.mode.get() == Mode::Hint {
            self.set_mode(Mode::Normal);
        }
    }

    fn enter_command_with(&self, initial: &str) {
        self.exit_hint_mode_if_active();
        self.mode.set(Mode::Command);
        self.key_handler.reset_pending();
        *self.pending_keys.borrow_mut() = None;
        self.command_bar.enter_command_mode_with(initial);
        self.sync_chrome();
    }

    fn handle_action(&self, action: BrowserAction) {
        match action {
            BrowserAction::EnterMode(mode) => self.set_mode(mode),
            BrowserAction::EnterCommandWith(prefix) => self.enter_command_with(&prefix),
            BrowserAction::StartHints => {
                self.set_mode(Mode::Hint);
                self.hints.start(&self.tab_view);
            }
            BrowserAction::HintChar(ch) => {
                let typed = self.hints.push_char(&self.tab_view, ch);
                *self.pending_keys.borrow_mut() = Some(typed.clone());
                self.status_bar.set_pending_keys(Some(&typed));
            }
            BrowserAction::HintBackspace => match self.hints.pop_char(&self.tab_view) {
                Some(remaining) => {
                    let display = (!remaining.is_empty()).then_some(remaining);
                    *self.pending_keys.borrow_mut() = display.clone();
                    self.status_bar.set_pending_keys(display.as_deref());
                }
                None => self.set_mode(Mode::Normal),
            },
            BrowserAction::ScrollDown => self.with_active_view(scroll::scroll_down),
            BrowserAction::ScrollUp => self.with_active_view(scroll::scroll_up),
            BrowserAction::ScrollLeft => self.with_active_view(scroll::scroll_left),
            BrowserAction::ScrollRight => self.with_active_view(scroll::scroll_right),
            BrowserAction::ScrollTop => self.with_active_view(scroll::scroll_top),
            BrowserAction::ScrollBottom => self.with_active_view(scroll::scroll_bottom),
            BrowserAction::NextTab => self.tab_view.next_tab(),
            BrowserAction::PrevTab => self.tab_view.prev_tab(),
            BrowserAction::OpenInCurrentTab(url) => self.tab_view.load_url_in_active_tab(&url),
            BrowserAction::OpenInNewTab(url) => self.tab_view.open_url_in_new_tab(&url),
            BrowserAction::CloseTab => {
                if !self.tab_view.close_active_tab() {
                    self.window.close();
                }
            }
            BrowserAction::Reload => self.tab_view.reload_active_tab(),
            BrowserAction::GoBack => self.with_active_view(|v| v.go_back()),
            BrowserAction::GoForward => self.with_active_view(|v| v.go_forward()),
            BrowserAction::SetZoom(level) => self.with_active_view(|v| v.set_zoom_level(level)),
            BrowserAction::ShowHelp => {
                self.tab_view
                    .load_html_in_active_tab("nservo — Help", start_page::HELP_PAGE_HTML);
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
            BrowserAction::Quit => self.window.close(),
        }
    }

    fn handle_hint_event(&self, tab_id: u64, event: HintEvent) {
        if self.mode.get() != Mode::Hint || self.tab_view.active_tab_id() != Some(tab_id) {
            return;
        }
        match event {
            HintEvent::Shown { .. } => {}
            HintEvent::Activated { is_input } => {
                self.set_mode(if is_input { Mode::Input } else { Mode::Normal });
            }
            HintEvent::Cancelled => self.set_mode(Mode::Normal),
            HintEvent::Empty => {
                self.set_mode(Mode::Normal);
                self.status_bar.show_notice("No clickable elements found");
            }
        }
    }

    fn with_active_view(&self, f: impl FnOnce(&servo_gtk::WebView)) {
        if let Some(view) = self.tab_view.active_web_view() {
            f(&view);
        }
    }

    fn sync_chrome(&self) {
        let (tabs, active_index, zoom_level) = self.tab_view.snapshot();
        self.tab_bar.render(&tabs, active_index);

        let active_tab = tabs.get(active_index);
        let window_title = match active_tab {
            Some(tab) => format!("{} — nservo", tab.display_title()),
            None => "nservo".to_string(),
        };
        self.window.set_title(Some(&window_title));

        let pending = self.pending_keys.borrow().clone();
        self.status_bar.update(&StatusSnapshot {
            mode: self.mode.get(),
            tab: active_tab,
            active_index,
            tab_count: tabs.len(),
            zoom_level,
            pending_keys: pending.as_deref(),
        });
    }
}
