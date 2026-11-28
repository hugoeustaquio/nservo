use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;

use crate::core::TabInfo;

const MAX_TAB_TITLE_CHARS: usize = 28;
const TAB_SCROLL_MARGIN_PX: f64 = 24.0;
const TAB_WHEEL_STEP_PX: f64 = 80.0;

/// Top Neovim-style tabline displaying all open tabs and highlighting the active one.
#[derive(Clone)]
pub struct TabBar {
    container: gtk::Box,
    tabs_box: gtk::Box,
    scroller: gtk::ScrolledWindow,
    buttons: Rc<RefCell<Vec<gtk::Button>>>,
    last_focus_state: Rc<Cell<Option<(usize, usize)>>>,
    on_select: Rc<dyn Fn(usize)>,
}

impl TabBar {
    pub fn new(on_select: impl Fn(usize) + 'static) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        container.add_css_class("tab-bar");
        container.set_hexpand(true);

        let tabs_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::External)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .propagate_natural_width(false)
            .hexpand(true)
            .child(&tabs_box)
            .build();

        let left_btn = make_scroll_button("‹");
        let right_btn = make_scroll_button("›");

        let adj = scroller.hadjustment();
        wire_scroll_button(&left_btn, &adj, -1.0);
        wire_scroll_button(&right_btn, &adj, 1.0);
        wire_wheel_scroll(&scroller, &adj);
        wire_overflow_sync(&adj, &left_btn, &right_btn);

        container.append(&left_btn);
        container.append(&scroller);
        container.append(&right_btn);

        Self {
            container,
            tabs_box,
            scroller,
            buttons: Rc::new(RefCell::new(Vec::new())),
            last_focus_state: Rc::new(Cell::new(None)),
            on_select: Rc::new(on_select),
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }

    /// Update tab pills in place to avoid tearing down and recreating GTK widgets
    /// on every title, loading, or mode change.
    pub fn render(&self, tabs: &[TabInfo], active_index: usize) {
        let mut buttons = self.buttons.borrow_mut();

        while buttons.len() > tabs.len() {
            if let Some(btn) = buttons.pop() {
                self.tabs_box.remove(&btn);
            }
        }

        while buttons.len() < tabs.len() {
            let idx = buttons.len();
            let button = gtk::Button::new();
            button.add_css_class("tab-button");
            button.set_can_focus(false);
            let on_select = self.on_select.clone();
            button.connect_clicked(move |_| on_select(idx));
            self.tabs_box.append(&button);
            buttons.push(button);
        }

        for (idx, (button, tab)) in buttons.iter().zip(tabs.iter()).enumerate() {
            let label = format_tab_label(idx, tab);
            if button.label().as_deref() != Some(label.as_str()) {
                button.set_label(&label);
            }
            let is_active = idx == active_index;
            let has_active = button.has_css_class("tab-active");
            if is_active && !has_active {
                button.add_css_class("tab-active");
            } else if !is_active && has_active {
                button.remove_css_class("tab-active");
            }
        }

        let focus_state = Some((active_index, tabs.len()));
        if self.last_focus_state.replace(focus_state) != focus_state
            && let Some(active_btn) = buttons.get(active_index).cloned()
        {
            scroll_tab_into_view(&self.scroller, &self.tabs_box, active_btn);
        }
    }
}

fn make_scroll_button(label: &str) -> gtk::Button {
    let btn = gtk::Button::with_label(label);
    btn.add_css_class("tab-scroll-btn");
    btn.set_can_focus(false);
    btn.set_visible(false);
    btn
}

fn scroll_by(adj: &gtk::Adjustment, delta: f64) {
    let max_val = (adj.upper() - adj.page_size()).max(0.0);
    adj.set_value((adj.value() + delta).clamp(0.0, max_val));
}

fn wire_scroll_button(btn: &gtk::Button, adj: &gtk::Adjustment, dir: f64) {
    let adj = adj.clone();
    btn.connect_clicked(move |_| {
        let step = (adj.page_size() * 0.7).max(160.0);
        scroll_by(&adj, dir * step);
    });
}

fn wire_wheel_scroll(scroller: &gtk::ScrolledWindow, adj: &gtk::Adjustment) {
    let ctrl = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    let adj = adj.clone();
    ctrl.connect_scroll(move |_, _dx, dy| {
        if adj.upper() > adj.page_size() + 1.0 && dy.abs() > f64::EPSILON {
            scroll_by(&adj, dy * TAB_WHEEL_STEP_PX);
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    scroller.add_controller(ctrl);
}

fn wire_overflow_sync(adj: &gtk::Adjustment, left: &gtk::Button, right: &gtk::Button) {
    let sync = {
        let (left, right) = (left.clone(), right.clone());
        move |a: &gtk::Adjustment| {
            let max_val = (a.upper() - a.page_size()).max(0.0);
            let overflows = max_val > 1.0;
            left.set_visible(overflows);
            right.set_visible(overflows);
            left.set_sensitive(a.value() > 0.5);
            right.set_sensitive(a.value() < max_val - 0.5);
        }
    };
    let sync_val = sync.clone();
    adj.connect_changed(sync);
    adj.connect_value_changed(sync_val);
}

fn scroll_tab_into_view(scroller: &gtk::ScrolledWindow, tabs_box: &gtk::Box, btn: gtk::Button) {
    let adj = scroller.hadjustment();
    let tabs_box_ref = tabs_box.clone();
    // Wait one frame tick so GTK4 finishes laying out newly added tab pills
    // and updates `hadjustment.upper()` before computing bounds.
    let laid_out = Cell::new(false);
    tabs_box.add_tick_callback(move |_, _| {
        if !laid_out.replace(true) {
            return glib::ControlFlow::Continue;
        }
        let Some(bounds) = btn.compute_bounds(&tabs_box_ref) else {
            return glib::ControlFlow::Break;
        };
        let page = adj.page_size();
        let max_val = (adj.upper() - page).max(0.0);
        if page > 0.0 && max_val > 1.0 {
            let left = f64::from(bounds.x());
            let right = left + f64::from(bounds.width());
            let view_left = adj.value();
            let view_right = view_left + page;
            if left - TAB_SCROLL_MARGIN_PX < view_left {
                adj.set_value((left - TAB_SCROLL_MARGIN_PX).max(0.0));
            } else if right + TAB_SCROLL_MARGIN_PX > view_right {
                adj.set_value((right + TAB_SCROLL_MARGIN_PX - page).clamp(0.0, max_val));
            }
        }
        glib::ControlFlow::Break
    });
}

fn format_tab_label(index: usize, tab: &TabInfo) -> String {
    let loading_mark = if tab.is_loading { "* " } else { "" };
    let title = truncate_chars(tab.display_title(), MAX_TAB_TITLE_CHARS);
    format!("[{}] {loading_mark}{title}", index + 1)
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let collected: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{collected}…")
    } else {
        collected
    }
}
