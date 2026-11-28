use std::rc::Rc;

use crate::core::{
    BrowserConfig, ConfigEvent, HintEvent, Mode, PluginAction, SearchEvent, default_config_path,
    parse_command,
};
use crate::engine::{clipboard, config_page};
use crate::ui::window::BrowserWindow;

impl BrowserWindow {
    pub(super) fn reload_plugins(&self) {
        let list = self.plugins.reload();
        let mut cfg = BrowserConfig::load_default();
        cfg.apply_plugin_defaults(&list);
        *self.config.borrow_mut() = cfg.clone();
        self.key_handler.set_config(cfg);
    }

    pub(super) fn apply_plugin_actions(self: &Rc<Self>, actions: Vec<PluginAction>) {
        for act in actions {
            match act {
                PluginAction::EvalJs(code) => {
                    self.with_active_view(|v| v.evaluate_javascript(&code))
                }
                PluginAction::InsertText(text) => {
                    if let Some(tab) = self.tab_view.active_tab() {
                        tab.insert_utf16_text(&text);
                    }
                }
                PluginAction::Copy(text) => clipboard::write_system_clipboard(&text),
                PluginAction::Open { url, new_tab: true } => {
                    self.tab_view.open_url_in_new_tab(&url)
                }
                PluginAction::Open {
                    url,
                    new_tab: false,
                } => self.tab_view.load_url_in_active_tab(&url),
                PluginAction::Command(cmd) => {
                    if let Ok(browser_act) = parse_command(&cmd) {
                        self.handle_action(browser_act);
                    }
                }
                PluginAction::Notice(msg) => self.status_bar.show_notice(&msg),
                PluginAction::Error(msg) => self.status_bar.show_error(&msg),
            }
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

    pub(super) fn handle_config_event(&self, tab_id: u64, event: ConfigEvent) {
        let path = default_config_path();
        let (mut next_cfg, reset) = match event {
            ConfigEvent::Save(cfg) => (cfg, false),
            ConfigEvent::Reset => (BrowserConfig::default(), true),
        };
        if reset {
            next_cfg.apply_plugin_defaults(&self.plugins.installed());
        }
        match next_cfg.save(&path) {
            Ok(()) => {
                servo_gtk::set_user_agent(next_cfg.user_agent.clone());
                *self.config.borrow_mut() = next_cfg.clone();
                self.key_handler.set_config(next_cfg.clone());
                let msg = if reset {
                    format!("Reset config to defaults ({})", path.display())
                } else {
                    format!("Saved config to {}", path.display())
                };
                self.status_bar.show_notice(&msg);
                if reset && self.tab_view.active_tab_id() == Some(tab_id) {
                    let html = config_page::render_config_html(
                        &next_cfg,
                        &path,
                        &self.plugins.installed(),
                    );
                    self.tab_view.load_html_in_active_tab(
                        config_page::CONFIG_PAGE_URL,
                        "nservo — Config",
                        &html,
                    );
                } else if let Some(tab) = self.tab_view.active_tab()
                    && self.tab_view.active_tab_id() == Some(tab_id)
                {
                    tab.notify_config_result(true, &msg);
                }
            }
            Err(err) => {
                self.status_bar.show_error(&err);
                if let Some(tab) = self.tab_view.active_tab()
                    && self.tab_view.active_tab_id() == Some(tab_id)
                {
                    tab.notify_config_result(false, &err);
                }
            }
        }
    }
}
