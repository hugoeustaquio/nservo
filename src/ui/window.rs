use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::prelude::*;

use crate::core::{ConsoleStore, Mode, parse_command};
use crate::engine::gpu::{self, RenderBackend};
use crate::ui::bookmark_controller::BookmarkController;
use crate::ui::command_bar::CommandBar;
use crate::ui::hint_controller::HintController;
use crate::ui::history_controller::HistoryController;
use crate::ui::inspector_panel::{InspectorCallbacks, InspectorPanel};
use crate::ui::key_handler::{KeyCallbacks, KeyHandler};
use crate::ui::search_controller::SearchController;
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
    pub(super) inspector: InspectorPanel,
    pub(super) console_store: RefCell<ConsoleStore>,
    pub(super) last_inspected_tab: Cell<Option<u64>>,
    pub(super) status_bar: StatusBar,
    pub(super) command_bar: CommandBar,
    pub(super) key_handler: KeyHandler,
    pub(super) sessions: SessionController,
    pub(super) bookmarks: BookmarkController,
    pub(super) history: HistoryController,
    pub(super) hints: HintController,
    pub(super) search: SearchController,
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
            let (s_sub, s_cmp) = (controller_slot.clone(), controller_slot.clone());
            CommandBar::new(
                move |raw| { if let Some(c) = get_ctrl(&s_sub) { c.submit_prompt(&raw); } },
                move || get_ctrl(&s_cmp).map(|c| c.load_completion_sources()).unwrap_or_default(),
            )
        };

        let inspector = {
            let s_eval = controller_slot.clone();
            let s_ref = controller_slot.clone();
            let s_sel = controller_slot.clone();
            let s_pick = controller_slot.clone();
            let s_clr = controller_slot.clone();
            let s_cls = controller_slot.clone();
            InspectorPanel::new(InspectorCallbacks {
                on_eval_js: Rc::new(move |expr| {
                    if let Some(c) = get_ctrl(&s_eval) { c.eval_inspector_js(&expr); }
                }),
                on_refresh_dom: Rc::new(move || {
                    if let Some(c) = get_ctrl(&s_ref) { c.refresh_inspector_dom(); }
                }),
                on_select_dom_node: Rc::new(move |idx| {
                    if let Some(c) = get_ctrl(&s_sel) { c.inspect_dom_node(idx); }
                }),
                on_toggle_picker: Rc::new(move || {
                    if let Some(c) = get_ctrl(&s_pick) { c.toggle_element_picker(); }
                }),
                on_clear_console: Rc::new(move || {
                    if let Some(c) = get_ctrl(&s_clr) { c.clear_active_console(); }
                }),
                on_close: Rc::new(move || {
                    if let Some(c) = get_ctrl(&s_cls) { c.set_inspector_open(false); }
                }),
            })
        };

        let tab_view = {
            let s_ch = controller_slot.clone();
            let s_pop = controller_slot.clone();
            let s_hint = controller_slot.clone();
            let s_ins = controller_slot.clone();
            let s_srch = controller_slot.clone();
            let s_hist = controller_slot.clone();
            let s_nav = controller_slot.clone();
            TabView::new(TabViewCallbacks {
                on_state_changed: Rc::new(move || {
                    if let Some(c) = get_ctrl(&s_ch) { c.sync_chrome(); }
                }),
                on_popup_url: Rc::new(move |url| {
                    if let Some(c) = get_ctrl(&s_pop) { c.tab_view.open_url_in_new_tab(&url); }
                }),
                on_hint_event: Rc::new(move |id, ev| {
                    if let Some(c) = get_ctrl(&s_hint) { c.handle_hint_event(id, ev); }
                }),
                on_inspector_event: Rc::new(move |id, ev| {
                    if let Some(c) = get_ctrl(&s_ins) { c.handle_inspector_event(id, ev); }
                }),
                on_search_event: Rc::new(move |id, ev| {
                    if let Some(c) = get_ctrl(&s_srch) { c.handle_search_event(id, ev); }
                }),
                on_history_sync: Rc::new(move |sync| {
                    if let Some(c) = get_ctrl(&s_hist) { c.history.handle_sync(sync); }
                }),
                on_navigation: Rc::new(move |id| {
                    if let Some(c) = get_ctrl(&s_nav) { c.handle_navigation(id); }
                }),
            })
        };

        let content_paned = gtk::Paned::new(gtk::Orientation::Vertical);
        content_paned.set_start_child(Some(tab_view.widget()));
        content_paned.set_end_child(Some(inspector.widget()));
        content_paned.set_resize_start_child(true);
        content_paned.set_resize_end_child(false);
        content_paned.set_shrink_start_child(false);
        content_paned.set_shrink_end_child(false);
        content_paned.set_vexpand(true);

        let layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        layout.append(tab_bar.widget());
        layout.append(&content_paned);
        layout.append(status_bar.widget());
        layout.append(command_bar.widget());
        window.set_child(Some(&layout));

        let controller = Rc::new(Self {
            window: window.clone(),
            tab_bar,
            tab_view,
            inspector,
            console_store: RefCell::new(ConsoleStore::default()),
            last_inspected_tab: Cell::new(None),
            status_bar,
            command_bar,
            key_handler,
            sessions: SessionController::new(),
            bookmarks: BookmarkController::new(),
            history: HistoryController::new(),
            hints: HintController::new(),
            search: SearchController::new(),
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
        let inspector = self.inspector.clone();

        self.key_handler.attach(
            &self.window,
            KeyCallbacks {
                on_action: Rc::new(move |action| on_action_ctrl.handle_action(action)),
                on_pending_changed: Rc::new(move |pending| {
                    on_pending_ctrl.set_pending_display(pending);
                }),
                is_command_entry_empty: Rc::new(move || cmd_bar.is_entry_empty()),
                is_inspector_focused: Rc::new(move || inspector.has_focus_within()),
            },
        );
    }

    pub(super) fn set_pending_display(&self, pending: Option<String>) {
        *self.pending_keys.borrow_mut() = pending.clone();
        self.status_bar.set_pending_keys(pending.as_deref());
    }

    fn submit_prompt(&self, raw: &str) {
        let mode = self.mode.get();
        self.set_mode(Mode::Normal);
        if mode == Mode::Search {
            self.search.submit(&self.tab_view, &self.status_bar, raw);
            return;
        }
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
        self.key_handler.set_input_active(next == Mode::Insert);
        *self.pending_keys.borrow_mut() = None;

        match next {
            Mode::Normal | Mode::Insert | Mode::Hint => {
                self.command_bar.exit_command_mode();
                self.tab_view.focus_active();
            }
            Mode::Command => self.command_bar.enter_command_mode_with(command_prefix),
            Mode::Search => self.command_bar.enter_search_mode(),
        }

        self.sync_chrome();
    }

    fn exit_hint_mode_if_active(&self) {
        if self.mode.get() == Mode::Hint {
            self.set_mode(Mode::Normal);
        }
    }

    pub(super) fn sync_chrome(&self) {
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
        self.sync_inspector_for_active_tab(active_tab.map(|t| t.id));
    }
}
