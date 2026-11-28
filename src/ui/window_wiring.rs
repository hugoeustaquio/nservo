use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::command_bar::CommandBar;
use crate::ui::inspector_panel::{InspectorCallbacks, InspectorPanel};
use crate::ui::tab_bar::TabBar;
use crate::ui::tab_view::{TabView, TabViewCallbacks};
use crate::ui::window::BrowserWindow;

pub(super) type WindowSlot = Rc<RefCell<Option<Rc<BrowserWindow>>>>;

pub(super) fn get_ctrl(slot: &WindowSlot) -> Option<Rc<BrowserWindow>> {
    slot.borrow().clone()
}

pub(super) fn build_tab_bar(slot: &WindowSlot) -> TabBar {
    let slot = slot.clone();
    TabBar::new(move |index| {
        if let Some(ctrl) = get_ctrl(&slot) {
            ctrl.exit_hint_mode_if_active();
            ctrl.tab_view.select_index(index);
        }
    })
}

pub(super) fn build_command_bar(slot: &WindowSlot) -> CommandBar {
    let (s_sub, s_cmp) = (slot.clone(), slot.clone());
    CommandBar::new(
        move |raw| {
            if let Some(c) = get_ctrl(&s_sub) {
                c.submit_prompt(&raw);
            }
        },
        move || {
            get_ctrl(&s_cmp)
                .map(|c| c.load_completion_sources())
                .unwrap_or_default()
        },
    )
}

pub(super) fn build_inspector(slot: &WindowSlot) -> InspectorPanel {
    let (s_eval, s_ref, s_dbg) = (slot.clone(), slot.clone(), slot.clone());
    let (s_sel, s_pick, s_clr, s_cls) = (slot.clone(), slot.clone(), slot.clone(), slot.clone());
    InspectorPanel::new(InspectorCallbacks {
        on_eval_js: Rc::new(move |expr| {
            if let Some(c) = get_ctrl(&s_eval) {
                c.eval_inspector_js(&expr);
            }
        }),
        on_refresh_dom: Rc::new(move || {
            if let Some(c) = get_ctrl(&s_ref) {
                c.refresh_inspector_dom();
            }
        }),
        on_open_debugger: Rc::new(move || {
            if let Some(c) = get_ctrl(&s_dbg) {
                c.connect_debugger_for_active_tab();
            }
        }),
        on_select_dom_node: Rc::new(move |idx| {
            if let Some(c) = get_ctrl(&s_sel) {
                c.inspect_dom_node(idx);
            }
        }),
        on_toggle_picker: Rc::new(move || {
            if let Some(c) = get_ctrl(&s_pick) {
                c.toggle_element_picker();
            }
        }),
        on_clear_console: Rc::new(move || {
            if let Some(c) = get_ctrl(&s_clr) {
                c.clear_active_console();
            }
        }),
        on_close: Rc::new(move || {
            if let Some(c) = get_ctrl(&s_cls) {
                c.set_inspector_open(false);
            }
        }),
    })
}

pub(super) fn build_tab_view(slot: &WindowSlot) -> TabView {
    let (s_ch, s_pop, s_hint, s_ins) = (slot.clone(), slot.clone(), slot.clone(), slot.clone());
    let (s_srch, s_cfg, s_hist, s_nav) = (slot.clone(), slot.clone(), slot.clone(), slot.clone());
    TabView::new(TabViewCallbacks {
        on_state_changed: Rc::new(move || {
            if let Some(c) = get_ctrl(&s_ch) {
                c.sync_chrome();
            }
        }),
        on_popup_url: Rc::new(move |url| {
            if let Some(c) = get_ctrl(&s_pop) {
                c.tab_view.open_url_in_new_tab(&url);
            }
        }),
        on_hint_event: Rc::new(move |id, ev| {
            if let Some(c) = get_ctrl(&s_hint) {
                c.handle_hint_event(id, ev);
            }
        }),
        on_inspector_event: Rc::new(move |id, ev| {
            if let Some(c) = get_ctrl(&s_ins) {
                c.handle_inspector_event(id, ev);
            }
        }),
        on_search_event: Rc::new(move |id, ev| {
            if let Some(c) = get_ctrl(&s_srch) {
                c.handle_search_event(id, ev);
            }
        }),
        on_config_event: Rc::new(move |id, ev| {
            if let Some(c) = get_ctrl(&s_cfg) {
                c.handle_config_event(id, ev);
            }
        }),
        on_history_sync: Rc::new(move |sync| {
            if let Some(c) = get_ctrl(&s_hist) {
                c.history.handle_sync(sync);
            }
        }),
        on_navigation: Rc::new(move |id| {
            if let Some(c) = get_ctrl(&s_nav) {
                c.handle_navigation(id);
            }
        }),
    })
}
