use std::path::PathBuf;
use crate::error::Result;
use crate::session::model::{SessionData, TabSessionData};
use crate::tabs::TabManager;

pub struct SessionStorage {
    path: PathBuf,
}

impl SessionStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn save_from_tabs(&self, tab_mgr: &TabManager) -> Result<()> {
        let mut tab_sessions = Vec::new();
        for tab in tab_mgr.tabs() {
            let (sx, sy) = tab.get_scroll();
            tab_sessions.push(TabSessionData {
                url: tab.url.clone(),
                title: tab.title.clone(),
                scroll_x: sx,
                scroll_y: sy,
            });
        }

        let session = SessionData::new(tab_mgr.active_index(), tab_sessions);
        self.save(&session)
    }

    pub fn save(&self, session: &SessionData) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(session)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn load(&self) -> Result<Option<SessionData>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&self.path)?;
        let session: SessionData = serde_json::from_str(&content)?;
        Ok(Some(session))
    }

    pub fn restore_into_tabs(&self, tab_mgr: &mut TabManager) -> Result<bool> {
        if let Some(session) = self.load()? {
            if session.tabs.is_empty() {
                return Ok(false);
            }

            for tab_data in &session.tabs {
                let _idx = tab_mgr.open_tab(&tab_data.url)?;
                if let Some(tab) = tab_mgr.current_tab_mut() {
                    tab.set_scroll(tab_data.scroll_x, tab_data.scroll_y);
                }
            }

            let _ = tab_mgr.set_active_index(session.active_tab_index);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
