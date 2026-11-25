use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::prelude::*;

use crate::core::{Mode, parse_command};
use crate::engine::gpu::{self, RenderBackend};
use crate::ui::command_bar::CommandBar;
use crate::ui::hint_controller::HintController;
use crate::ui::key_handler::{KeyCallbacks, KeyHandler};
use crate::ui::session_controller::SessionController;
use crate::ui::status_bar::{StatusBar, StatusSnapshot};
use crate::ui::tab_bar::TabBar;
use crate::ui::tab_view::{TabView, TabViewCallbacks};

type WindowSlot = Rc<RefCell<Option<Rc<BrowserWindow>>>>;

fn get_ctrl(slot: &WindowSlot) -> Option<Rc<BrowserWindow>> {
    slot.borrow().clone()
}

/// Top-level Neovim-inspired GTK4 browser window.
pub struct BrowserWindow {
    pub(super) window: gtk::ApplicationWindow,
    pub(super) tab_bar: TabBar,
    pub(super) tab_view: TabView,
    pub(super) status_bar: StatusBar,
    pub(super) command_bar: CommandBar,
    pub(super) key_handler: KeyHandler,
    pub(super) sessions: SessionController,
    pub(super) hints: HintController,
    pub(super) mode: Rc<Cell<Mode>>,
    pub(super) render_backend: Cell<RenderBackend>,
    pub(super) pending_keys: RefCell<Option<String>>,
}

impl BrowserWindow {
    /// Build, wire, and present a new browser window.
    pub fn open(app: &gtk::Application, initial_url: Option<&str>, initial_backend: RenderBackend) {
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
        let controller_slot: WindowSlot = Rc::new(RefCell::new(None));

        let tab_bar = {
            let slot = controller_slot.clone();
            TabBar::new(move |index| {
                if let Some(ctrl) = get_ctrl(&slot) {
                    ctrl.exit_hint_mode_if_active();
                    ctrl.tab_view.select_index(index);
                }
            })
        };

        let command_bar = {
            let slot = controller_slot.clone();
            CommandBar::new(move |raw| {
                if let Some(ctrl) = get_ctrl(&slot) {
                    ctrl.submit_command(&raw);
                }
            })
        };

        let tab_view = {
            let (s_ch, s_pop, s_hint) = (
                controller_slot.clone(),
                controller_slot.clone(),
                controller_slot.clone(),
            );
            TabView::new(TabViewCallbacks {
                on_state_changed: Rc::new(move || {
                    if let Some(ctrl) = get_ctrl(&s_ch) {
                        ctrl.sync_chrome();
                    }
                }),
                on_popup_url: Rc::new(move |url| {
                    if let Some(ctrl) = get_ctrl(&s_pop) {
                        ctrl.tab_view.open_url_in_new_tab(&url);
                    }
                }),
                on_hint_event: Rc::new(move |tab_id, event| {
                    if let Some(ctrl) = get_ctrl(&s_hint) {
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
            render_backend: Cell::new(initial_backend),
            pending_keys: RefCell::new(None),
        });

        *controller_slot.borrow_mut() = Some(controller.clone());
        controller.attach_key_handler();
        let slot_gpu = controller_slot.clone();
        gpu::watch_window_backend(&window, move |backend| {
            if let Some(ctrl) = get_ctrl(&slot_gpu) {
                ctrl.render_backend.set(backend);
                ctrl.sync_chrome();
            }
        });

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
                    on_pending_ctrl.set_pending_display(pending);
                }),
                is_command_entry_empty: Rc::new(move || cmd_bar.is_entry_empty()),
            },
        );
    }

    pub(super) fn set_pending_display(&self, pending: Option<String>) {
        *self.pending_keys.borrow_mut() = pending.clone();
        self.status_bar.set_pending_keys(pending.as_deref());
    }

    fn submit_command(&self, raw: &str) {
        self.set_mode(Mode::Normal);
        match parse_command(raw) {
            Ok(action) => self.handle_action(action),
            Err(err) => self.status_bar.show_error(&err.to_string()),
        }
    }

    pub(super) fn set_mode(&self, next: Mode) {
        self.transition_mode(next, "");
    }

    pub(super) fn transition_mode(&self, next: Mode, command_prefix: &str) {
        if self.mode.get() == Mode::Hint && next != Mode::Hint {
            self.hints.clear(&self.tab_view);
        }

        self.mode.set(next);
        self.key_handler.reset_pending();
        self.key_handler.set_input_active(next == Mode::Input);
        *self.pending_keys.borrow_mut() = None;

        match next {
            Mode::Normal | Mode::Input | Mode::Hint => {
                self.command_bar.exit_command_mode();
                self.tab_view.focus_active();
            }
            Mode::Command => self.command_bar.enter_command_mode_with(command_prefix),
        }

        self.sync_chrome();
    }

    fn exit_hint_mode_if_active(&self) {
        if self.mode.get() == Mode::Hint {
            self.set_mode(Mode::Normal);
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
            render_backend: self.render_backend.get(),
        });
    }
}
