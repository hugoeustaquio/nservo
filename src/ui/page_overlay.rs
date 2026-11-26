use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use crate::core::ScrollMetrics;
use crate::engine::web_tab::{WeakWebTab, WebTab};

const HIDE_DELAY: Duration = Duration::from_millis(1200);
const WHEEL_STEP_PX: f64 = 90.0;
const EDGE_REVEAL_PX: f64 = 20.0;

#[derive(Clone)]
struct AxisBar {
    bar: gtk::Scrollbar,
    adj: gtk::Adjustment,
    scrollable: Rc<Cell<bool>>,
    hovered: Rc<Cell<bool>>,
    syncing: Rc<Cell<bool>>,
    hide_gen: Rc<Cell<u64>>,
}

impl AxisBar {
    fn new(orientation: gtk::Orientation, tab_weak: WeakWebTab) -> Self {
        let adj = gtk::Adjustment::new(0.0, 0.0, 1.0, 40.0, 1.0, 1.0);
        let bar = gtk::Scrollbar::new(orientation, Some(&adj));
        bar.add_css_class("page-scrollbar");
        bar.set_can_focus(false);
        bar.set_visible(false);
        let (halign, valign) = match orientation {
            gtk::Orientation::Vertical => (gtk::Align::End, gtk::Align::Fill),
            _ => (gtk::Align::Fill, gtk::Align::End),
        };
        bar.set_halign(halign);
        bar.set_valign(valign);

        let axis = Self {
            bar,
            adj,
            scrollable: Rc::new(Cell::new(false)),
            hovered: Rc::new(Cell::new(false)),
            syncing: Rc::new(Cell::new(false)),
            hide_gen: Rc::new(Cell::new(0)),
        };

        // Dragging or clicking the GTK4 scrollbar scrolls the page 1:1.
        let syncing = axis.syncing.clone();
        let is_vertical = orientation == gtk::Orientation::Vertical;
        let reveal_bar = axis.bar.downgrade();
        let reveal_hovered = axis.hovered.clone();
        let reveal_gen = axis.hide_gen.clone();
        axis.adj.connect_value_changed(move |adj| {
            if syncing.get() {
                return;
            }
            reveal_weak(&reveal_bar, &reveal_hovered, &reveal_gen);
            if let Some(tab) = tab_weak.upgrade() {
                if is_vertical {
                    tab.jump_scroll_y(adj.value());
                } else {
                    tab.jump_scroll_x(adj.value());
                }
            }
        });

        // Hovering over the scrollbar expands it and keeps it visible.
        let motion = gtk::EventControllerMotion::new();
        let bar_enter = axis.bar.downgrade();
        let hovered_enter = axis.hovered.clone();
        let gen_enter = axis.hide_gen.clone();
        motion.connect_enter(move |_, _, _| {
            hovered_enter.set(true);
            gen_enter.set(gen_enter.get().wrapping_add(1));
            if let Some(bar) = bar_enter.upgrade() {
                bar.add_css_class("visible");
                bar.add_css_class("hovering");
            }
        });

        let bar_leave = axis.bar.downgrade();
        let hovered_leave = axis.hovered.clone();
        let gen_leave = axis.hide_gen.clone();
        motion.connect_leave(move |_| {
            hovered_leave.set(false);
            if let Some(bar) = bar_leave.upgrade() {
                bar.remove_css_class("hovering");
                schedule_hide(&bar, &hovered_leave, &gen_leave);
            }
        });
        axis.bar.add_controller(motion);

        axis
    }

    fn update(&self, value: f64, view_size: f64, total_size: f64, scrollable: bool) {
        self.scrollable.set(scrollable);
        if !scrollable {
            self.bar.set_visible(false);
            self.bar.remove_css_class("visible");
            return;
        }
        let changed = (self.adj.value() - value).abs() > 0.5;
        self.syncing.set(true);
        self.adj.configure(
            value.clamp(0.0, (total_size - view_size).max(0.0)),
            0.0,
            total_size,
            40.0,
            (view_size * 0.9).max(1.0),
            view_size.max(1.0),
        );
        self.syncing.set(false);
        self.bar.set_visible(true);
        if changed || self.hovered.get() {
            self.reveal();
        }
    }

    fn reveal(&self) {
        if !self.scrollable.get() {
            return;
        }
        self.bar.add_css_class("visible");
        schedule_hide(&self.bar, &self.hovered, &self.hide_gen);
    }

    fn downgrade(&self) -> WeakAxisBar {
        WeakAxisBar {
            bar: self.bar.downgrade(),
            adj: self.adj.downgrade(),
            scrollable: self.scrollable.clone(),
            hovered: self.hovered.clone(),
            syncing: self.syncing.clone(),
            hide_gen: self.hide_gen.clone(),
        }
    }
}

#[derive(Clone)]
struct WeakAxisBar {
    bar: glib::WeakRef<gtk::Scrollbar>,
    adj: glib::WeakRef<gtk::Adjustment>,
    scrollable: Rc<Cell<bool>>,
    hovered: Rc<Cell<bool>>,
    syncing: Rc<Cell<bool>>,
    hide_gen: Rc<Cell<u64>>,
}

impl WeakAxisBar {
    fn upgrade(&self) -> Option<AxisBar> {
        Some(AxisBar {
            bar: self.bar.upgrade()?,
            adj: self.adj.upgrade()?,
            scrollable: self.scrollable.clone(),
            hovered: self.hovered.clone(),
            syncing: self.syncing.clone(),
            hide_gen: self.hide_gen.clone(),
        })
    }
}

/// Wraps a [`WebTab`]'s `WebView` with auto-hiding, hover-expanding GTK4
/// overlay scrollbars and smooth discrete mouse-wheel scrolling.
#[derive(Clone)]
pub struct PageOverlay {
    overlay: gtk::Overlay,
    v_bar: AxisBar,
    h_bar: AxisBar,
}

/// Non-owning handle used by `UserContentManager` callbacks to avoid cycles.
#[derive(Clone)]
pub struct WeakPageOverlay {
    v_bar: WeakAxisBar,
    h_bar: WeakAxisBar,
}

impl WeakPageOverlay {
    pub fn update_metrics(&self, m: &ScrollMetrics) {
        if let Some(v) = self.v_bar.upgrade() {
            v.update(m.y, m.view_h, m.scroll_h, m.is_v_scrollable());
        }
        if let Some(h) = self.h_bar.upgrade() {
            h.update(m.x, m.view_w, m.scroll_w, m.is_h_scrollable());
        }
    }
}

impl PageOverlay {
    pub fn new(web_tab: &WebTab) -> Self {
        let overlay = gtk::Overlay::new();
        overlay.set_hexpand(true);
        overlay.set_vexpand(true);
        overlay.set_child(Some(web_tab.view()));

        let v_bar = AxisBar::new(gtk::Orientation::Vertical, web_tab.downgrade());
        let h_bar = AxisBar::new(gtk::Orientation::Horizontal, web_tab.downgrade());
        overlay.add_overlay(&v_bar.bar);
        overlay.add_overlay(&h_bar.bar);

        // Reveal overlay scrollbar when the pointer approaches the right/bottom edge.
        let edge_motion = gtk::EventControllerMotion::new();
        let overlay_weak = overlay.downgrade();
        let v_weak = v_bar.downgrade();
        let h_weak = h_bar.downgrade();
        edge_motion.connect_motion(move |_, x, y| {
            let Some(ov) = overlay_weak.upgrade() else {
                return;
            };
            let (w, h) = (f64::from(ov.width()), f64::from(ov.height()));
            if w - x <= EDGE_REVEAL_PX
                && let Some(v) = v_weak.upgrade()
            {
                v.reveal();
            }
            if h - y <= EDGE_REVEAL_PX
                && let Some(hb) = h_weak.upgrade()
            {
                hb.reveal();
            }
        });
        overlay.add_controller(edge_motion);

        // Smooth discrete mouse-wheel scrolling while letting smooth touchpad
        // surface gestures pass straight through to Servo.
        let scroll_ctrl =
            gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
        scroll_ctrl.set_propagation_phase(gtk::PropagationPhase::Capture);
        let tab_weak = web_tab.downgrade();
        scroll_ctrl.connect_scroll(move |ctrl, dx, dy| {
            if ctrl.unit() == gdk::ScrollUnit::Wheel
                && let Some(tab) = tab_weak.upgrade()
            {
                tab.smooth_scroll_by(dx * WHEEL_STEP_PX, dy * WHEEL_STEP_PX);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        overlay.add_controller(scroll_ctrl);

        Self {
            overlay,
            v_bar,
            h_bar,
        }
    }

    pub fn widget(&self) -> &gtk::Overlay {
        &self.overlay
    }

    pub fn downgrade(&self) -> WeakPageOverlay {
        WeakPageOverlay {
            v_bar: self.v_bar.downgrade(),
            h_bar: self.h_bar.downgrade(),
        }
    }
}

fn reveal_weak(
    bar_weak: &glib::WeakRef<gtk::Scrollbar>,
    hovered: &Rc<Cell<bool>>,
    hide_gen: &Rc<Cell<u64>>,
) {
    if let Some(bar) = bar_weak.upgrade() {
        bar.add_css_class("visible");
        schedule_hide(&bar, hovered, hide_gen);
    }
}

fn schedule_hide(bar: &gtk::Scrollbar, hovered: &Rc<Cell<bool>>, hide_gen: &Rc<Cell<u64>>) {
    let next_gen = hide_gen.get().wrapping_add(1);
    hide_gen.set(next_gen);

    let bar_weak = bar.downgrade();
    let hovered = hovered.clone();
    let hide_gen = hide_gen.clone();
    glib::timeout_add_local_once(HIDE_DELAY, move || {
        if hide_gen.get() != next_gen || hovered.get() {
            return;
        }
        if let Some(bar) = bar_weak.upgrade() {
            bar.remove_css_class("visible");
        }
    });
}
