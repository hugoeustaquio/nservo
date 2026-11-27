use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use gtk::glib;

use crate::core::{
    InstalledPlugin, PluginAction, PluginMode, default_config_path, default_plugins_dir,
    discover_plugins,
};
use crate::engine::plugin_runner::{
    PluginContext, build_page_js_payload, execute_plugin_command, install_plugin, remove_plugin,
    update_plugins,
};
use crate::ui::status_bar::StatusBar;
use crate::ui::tab_view::TabView;

const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Manages installed plugins under `$XDG_CONFIG_HOME/nservo/plugins/`, running
/// `page_js` scripts directly in Servo and `exec` binaries asynchronously.
pub struct PluginController {
    dir: PathBuf,
    installed: RefCell<Vec<InstalledPlugin>>,
}

impl PluginController {
    pub fn new() -> Self {
        let dir = default_plugins_dir();
        let installed = discover_plugins(&dir);
        Self {
            dir,
            installed: RefCell::new(installed),
        }
    }

    pub fn installed(&self) -> Vec<InstalledPlugin> {
        self.installed.borrow().clone()
    }

    pub fn reload(&self) -> Vec<InstalledPlugin> {
        let list = discover_plugins(&self.dir);
        *self.installed.borrow_mut() = list.clone();
        list
    }

    pub fn completion_entries(&self) -> Vec<(String, String)> {
        self.installed
            .borrow()
            .iter()
            .map(|p| (p.manifest.name.clone(), p.manifest.description.clone()))
            .collect()
    }

    pub fn run(
        &self,
        name: &str,
        args: &str,
        tab_view: &TabView,
        status_bar: &StatusBar,
        on_actions: Rc<dyn Fn(Vec<PluginAction>)>,
    ) {
        let cached = self
            .installed
            .borrow()
            .iter()
            .find(|p| p.manifest.name == name)
            .cloned();
        let plugin = cached.or_else(|| {
            self.reload()
                .into_iter()
                .find(|p| p.manifest.name == name)
        });

        let Some(plugin) = plugin else {
            status_bar.show_error(&format!("Plugin '{name}' not found in {}", self.dir.display()));
            return;
        };

        let info = tab_view.active_tab_info();
        let config_dir = default_config_path()
            .parent()
            .map(PathBuf::from)
            .unwrap_or_default();
        let ctx = PluginContext {
            url: info.as_ref().map(|t| t.url.clone()).unwrap_or_default(),
            title: info.as_ref().map(|t| t.title.clone()).unwrap_or_default(),
            args: args.to_string(),
            config_dir,
        };

        match plugin.manifest.mode {
            PluginMode::PageJs => match build_page_js_payload(&plugin, &ctx) {
                Ok(js) => {
                    if let Some(view) = tab_view.active_web_view() {
                        view.evaluate_javascript(&js);
                        status_bar.show_notice(&format!("Plugin: ran '{}'", plugin.manifest.name));
                    }
                }
                Err(err) => status_bar.show_error(&err),
            },
            PluginMode::Exec => {
                let pname = plugin.manifest.name.clone();
                status_bar.show_notice(&format!("Plugin: running '{pname}'…"));
                let (tx, rx) = mpsc::channel();
                thread::spawn(move || {
                    let _ = tx.send(execute_plugin_command(&plugin, &ctx));
                });
                let sb = status_bar.clone();
                glib::timeout_add_local(POLL_INTERVAL, move || match rx.try_recv() {
                    Ok(Ok(actions)) => {
                        on_actions(actions);
                        glib::ControlFlow::Break
                    }
                    Ok(Err(err)) => {
                        sb.show_error(&format!("Plugin '{pname}' failed: {err}"));
                        glib::ControlFlow::Break
                    }
                    Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
                });
            }
        }
    }

    pub fn install(
        &self,
        repo: &str,
        status_bar: &StatusBar,
        on_reloaded: Rc<dyn Fn()>,
    ) {
        let dir = self.dir.clone();
        let repo_owned = repo.to_string();
        status_bar.show_notice(&format!("Plugin: installing {repo}…"));
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(install_plugin(&dir, &repo_owned));
        });
        let sb = status_bar.clone();
        glib::timeout_add_local(POLL_INTERVAL, move || match rx.try_recv() {
            Ok(Ok(plugin)) => {
                on_reloaded();
                sb.show_notice(&format!("Plugin: installed '{}'", plugin.manifest.name));
                glib::ControlFlow::Break
            }
            Ok(Err(err)) => {
                sb.show_error(&format!("Plugin install failed: {err}"));
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
        });
    }

    pub fn update(
        &self,
        target: Option<&str>,
        status_bar: &StatusBar,
        on_reloaded: Rc<dyn Fn()>,
    ) {
        let dir = self.dir.clone();
        let target_owned = target.map(str::to_string);
        status_bar.show_notice("Plugin: updating…");
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(update_plugins(&dir, target_owned.as_deref()));
        });
        let sb = status_bar.clone();
        glib::timeout_add_local(POLL_INTERVAL, move || match rx.try_recv() {
            Ok(summary) => {
                on_reloaded();
                if summary.total == 0 {
                    sb.show_notice("Plugin: no matching plugins to update");
                } else if summary.failed.is_empty() {
                    sb.show_notice(&format!(
                        "Plugin: updated {}/{} plugin(s)",
                        summary.updated, summary.total
                    ));
                } else {
                    sb.show_error(&format!(
                        "Plugin: updated {}/{}, {} failed ({})",
                        summary.updated,
                        summary.total,
                        summary.failed.len(),
                        summary.failed[0].1
                    ));
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
        });
    }

    pub fn remove(&self, name: &str, status_bar: &StatusBar, on_reloaded: Rc<dyn Fn()>) {
        match remove_plugin(&self.dir, name) {
            Ok(removed) => {
                on_reloaded();
                status_bar.show_notice(&format!("Plugin: removed '{removed}'"));
            }
            Err(err) => status_bar.show_error(&err),
        }
    }
}
