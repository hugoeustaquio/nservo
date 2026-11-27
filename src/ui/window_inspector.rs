use crate::core::{ConsoleEntry, ConsoleLevel, InspectorEvent};
use crate::ui::window::BrowserWindow;

impl BrowserWindow {
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
            self.stop_element_picker();
            if let Some(tab) = self.tab_view.active_tab() {
                tab.clear_inspector_highlight();
            }
            self.tab_view.focus_active();
        }
    }

    pub(super) fn toggle_element_picker(&self) {
        if self.inspector.is_picker_active() {
            self.stop_element_picker();
            return;
        }
        self.inspector.set_picker_active(true);
        if let Some(tab) = self.tab_view.active_tab() {
            tab.start_element_picker();
        }
        self.tab_view.focus_active();
    }

    pub(super) fn stop_element_picker(&self) {
        if self.inspector.is_picker_active() {
            self.inspector.set_picker_active(false);
            if let Some(tab) = self.tab_view.active_tab() {
                tab.stop_element_picker();
            }
        }
    }

    pub(super) fn sync_inspector_for_active_tab(&self, active_id: Option<u64>) {
        if self.last_inspected_tab.get() == active_id {
            return;
        }
        self.last_inspected_tab.set(active_id);
        self.inspector.set_picker_active(false);
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
            InspectorEvent::DomTree(nodes) if is_active => {
                self.inspector.set_dom_nodes(&nodes);
            }
            InspectorEvent::ElementPicked {
                nodes,
                selected_index,
            } if is_active => {
                self.inspector.set_picker_active(false);
                self.inspector.show_dom_tab();
                self.inspector.set_dom_nodes(&nodes);
                self.inspector.select_dom_node(selected_index);
            }
            InspectorEvent::PickerCancelled if is_active => {
                self.inspector.set_picker_active(false);
            }
            InspectorEvent::ElementDetails(details) if is_active => {
                self.inspector.set_element_details(&details);
            }
            InspectorEvent::DomTree(_)
            | InspectorEvent::ElementPicked { .. }
            | InspectorEvent::PickerCancelled
            | InspectorEvent::ElementDetails(_) => {}
        }
    }
}
