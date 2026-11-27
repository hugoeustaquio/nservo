use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use gtk::glib;

use crate::core::{default_adblock_dir, list_filter_files};
use crate::engine::adblock_fetch::{download_filter_list, update_all_filter_lists};
use crate::ui::status_bar::StatusBar;
use crate::ui::tab_view::TabView;

const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Coordinates asynchronous `:adblock <URL>` downloads, `:adblock-update`, and
/// live rule reloading across all open Servo tabs.
pub struct AdblockController {
    dir: PathBuf,
}

impl AdblockController {
    pub fn new() -> Self {
        Self {
            dir: default_adblock_dir(),
        }
    }

    pub fn add_list(&self, url: &str, tab_view: &TabView, status_bar: &StatusBar) {
        let dir = self.dir.clone();
        let url_owned = url.to_string();
        status_bar.show_notice(&format!("Adblock: downloading {url}…"));

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = download_filter_list(&dir, &url_owned);
            let _ = tx.send(result);
        });

        let tab_view = tab_view.clone();
        let status_bar = status_bar.clone();
        let dir_str = self.dir.to_string_lossy().into_owned();
        glib::timeout_add_local(POLL_INTERVAL, move || match rx.try_recv() {
            Ok(Ok(summary)) => {
                tab_view.reload_adblock(&dir_str);
                status_bar.show_notice(&format!(
                    "Adblock: saved {} ({} rules)",
                    summary.url, summary.rule_count
                ));
                glib::ControlFlow::Break
            }
            Ok(Err(err)) => {
                status_bar.show_error(&format!("Adblock download failed: {err}"));
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
        });
    }

    pub fn update_lists(&self, tab_view: &TabView, status_bar: &StatusBar) {
        let updatable = list_filter_files(&self.dir)
            .into_iter()
            .filter(|(_, url)| url.is_some())
            .count();
        if updatable == 0 {
            status_bar.show_notice(&format!(
                "Adblock: no saved filter lists to update in {}",
                self.dir.display()
            ));
            return;
        }

        let dir = self.dir.clone();
        status_bar.show_notice(&format!("Adblock: updating {updatable} filter list(s)…"));

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let summary = update_all_filter_lists(&dir);
            let _ = tx.send(summary);
        });

        let tab_view = tab_view.clone();
        let status_bar = status_bar.clone();
        let dir_str = self.dir.to_string_lossy().into_owned();
        glib::timeout_add_local(POLL_INTERVAL, move || match rx.try_recv() {
            Ok(summary) => {
                tab_view.reload_adblock(&dir_str);
                if summary.failed.is_empty() {
                    status_bar.show_notice(&format!(
                        "Adblock: updated {}/{} list(s) ({} total rules)",
                        summary.updated, summary.total, summary.total_rules
                    ));
                } else {
                    let first_err = &summary.failed[0].1;
                    status_bar.show_error(&format!(
                        "Adblock: updated {}/{} list(s), {} failed ({first_err})",
                        summary.updated,
                        summary.total,
                        summary.failed.len()
                    ));
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
        });
    }
}
