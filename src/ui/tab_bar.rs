use std::rc::Rc;

use gtk::prelude::*;

use crate::core::TabInfo;

const MAX_TAB_TITLE_CHARS: usize = 28;

/// Top Neovim-style tabline displaying all open tabs and highlighting the active one.
#[derive(Clone)]
pub struct TabBar {
    container: gtk::Box,
    on_select: Rc<dyn Fn(usize)>,
}

impl TabBar {
    pub fn new(on_select: impl Fn(usize) + 'static) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        container.add_css_class("tab-bar");
        container.set_hexpand(true);

        Self {
            container,
            on_select: Rc::new(on_select),
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }

    /// Re-render tab pills to reflect current titles, loading states, and active tab.
    pub fn render(&self, tabs: &[TabInfo], active_index: usize) {
        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }

        for (idx, tab) in tabs.iter().enumerate() {
            let label = format_tab_label(idx, tab);
            let button = gtk::Button::with_label(&label);
            button.add_css_class("tab-button");
            button.set_can_focus(false);
            if idx == active_index {
                button.add_css_class("tab-active");
            }

            let on_select = self.on_select.clone();
            button.connect_clicked(move |_| on_select(idx));

            self.container.append(&button);
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
