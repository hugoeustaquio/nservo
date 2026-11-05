use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabSessionData {
    pub url: String,
    pub title: String,
    pub scroll_x: u32,
    pub scroll_y: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub active_tab_index: usize,
    pub tabs: Vec<TabSessionData>,
    pub saved_at: i64,
}

impl SessionData {
    pub fn new(active_tab_index: usize, tabs: Vec<TabSessionData>) -> Self {
        Self {
            active_tab_index,
            tabs,
            saved_at: chrono::Utc::now().timestamp(),
        }
    }
}
