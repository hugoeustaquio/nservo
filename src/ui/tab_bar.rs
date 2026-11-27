use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;

use crate::core::TabInfo;

const MAX_TAB_TITLE_CHARS: usize = 28;

/// Top Neovim-style tabline displaying all open tabs and highlighting the active one.
#[derive(Clone)]
pub struct TabBar {
    container: gtk::Box,
    buttons: Rc<RefCell<Vec<gtk::Button>>>,
    on_select: Rc<dyn Fn(usize)>,
}

impl TabBar {
    pub fn new(on_select: impl Fn(usize) + 'static) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        container.add_css_class("tab-bar");
        container.set_hexpand(true);

        Self {
            container,
            buttons: Rc::new(RefCell::new(Vec::new())),
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
                self.container.remove(&btn);
            }
        }

        while buttons.len() < tabs.len() {
            let idx = buttons.len();
            let button = gtk::Button::new();
            button.add_css_class("tab-button");
            button.set_can_focus(false);
            let on_select = self.on_select.clone();
            button.connect_clicked(move |_| on_select(idx));
            self.container.append(&button);
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
    }
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
