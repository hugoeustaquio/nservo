use crate::browser::{BrowserEngine, MockEngine, ServoEngine};
use crate::error::Result;

pub struct Tab {
    pub id: usize,
    pub title: String,
    pub url: String,
    pub engine: Box<dyn BrowserEngine>,
}

impl Tab {
    pub fn new_with_engine(id: usize, url: &str, engine: Box<dyn BrowserEngine>) -> Result<Self> {
        let mut tab = Self {
            id,
            title: "New Tab".to_string(),
            url: url.to_string(),
            engine,
        };
        tab.load_url(url)?;
        Ok(tab)
    }

    pub fn new(id: usize, url: &str) -> Result<Self> {
        Self::new_with_engine(id, url, Box::new(ServoEngine::new()))
    }

    pub fn new_mock(id: usize, url: &str) -> Result<Self> {
        Self::new_with_engine(id, url, Box::new(MockEngine::new()))
    }

    pub fn load_url(&mut self, url: &str) -> Result<()> {
        self.engine.load_url(url)?;
        self.url = self.engine.get_url();
        self.title = self.engine.get_title();
        Ok(())
    }

    pub fn get_scroll(&self) -> (u32, u32) {
        self.engine.get_scroll()
    }

    pub fn set_scroll(&mut self, x: u32, y: u32) {
        self.engine.scroll_to(x, y);
    }
}
