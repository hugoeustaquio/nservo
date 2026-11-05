use std::collections::VecDeque;
use crate::browser::dom::DomNode;
use crate::browser::event::{EngineEvent, NavigationState, ScrollPosition};
use crate::browser::layout::{LayoutEngine, LayoutNode};
use crate::browser::traits::BrowserEngine;
use crate::error::Result;

pub struct MockEngine {
    url: String,
    title: String,
    html: String,
    scroll: ScrollPosition,
    events: VecDeque<EngineEvent>,
    nav_state: NavigationState,
    history: Vec<String>,
    history_idx: usize,
    layout_nodes: Vec<LayoutNode>,
    content_height: usize,
}

impl MockEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            url: "about:blank".to_string(),
            title: "Blank Page".to_string(),
            html: "<html><head><title>Blank Page</title></head><body><h1>Blank</h1></body></html>".to_string(),
            scroll: ScrollPosition::new(0, 0),
            events: VecDeque::new(),
            nav_state: NavigationState::Idle,
            history: vec!["about:blank".to_string()],
            history_idx: 0,
            layout_nodes: Vec::new(),
            content_height: 720,
        };
        engine.rebuild_layout();
        engine
    }

    pub fn set_html(&mut self, html: &str) {
        self.html = html.to_string();
        self.rebuild_layout();
    }

    fn rebuild_layout(&mut self) {
        let dom = DomNode::parse_html(&self.html);
        if let Some(t) = dom.extract_title() {
            self.title = t;
        }
        let (nodes, total_h) = LayoutEngine::compute_layout(&dom, 1280);
        self.layout_nodes = nodes;
        self.content_height = total_h;
    }
}

impl Default for MockEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserEngine for MockEngine {
    fn load_url(&mut self, url: &str) -> Result<()> {
        self.url = url.to_string();
        self.title = format!("Page: {}", self.url);
        self.html = format!("<html><head><title>{}</title></head><body><h1>{}</h1><p>Test Content</p></body></html>", self.title, self.url);
        self.rebuild_layout();
        self.nav_state = NavigationState::Ready;
        self.history.push(self.url.clone());
        self.history_idx = self.history.len() - 1;

        self.events.push_back(EngineEvent::LoadStarted(self.url.clone()));
        self.events.push_back(EngineEvent::UrlChanged(self.url.clone()));
        self.events.push_back(EngineEvent::TitleChanged(self.title.clone()));
        self.events.push_back(EngineEvent::LoadFinished(self.url.clone()));
        Ok(())
    }

    fn reload(&mut self) -> Result<()> {
        self.load_url(&self.url.clone())
    }

    fn go_back(&mut self) -> Result<()> {
        if self.history_idx > 0 {
            self.history_idx -= 1;
            let target = self.history[self.history_idx].clone();
            self.load_url(&target)?;
        }
        Ok(())
    }

    fn go_forward(&mut self) -> Result<()> {
        if self.history_idx + 1 < self.history.len() {
            self.history_idx += 1;
            let target = self.history[self.history_idx].clone();
            self.load_url(&target)?;
        }
        Ok(())
    }

    fn eval_js(&mut self, script: &str) -> Result<String> {
        if script.contains("document.title") {
            Ok(self.title.clone())
        } else {
            Ok("ok".to_string())
        }
    }

    fn get_html(&self) -> String {
        self.html.clone()
    }

    fn get_title(&self) -> String {
        self.title.clone()
    }

    fn get_url(&self) -> String {
        self.url.clone()
    }

    fn scroll_by(&mut self, dx: i32, dy: i32) {
        self.scroll.x = (self.scroll.x as i32 + dx).max(0) as u32;
        self.scroll.y = (self.scroll.y as i32 + dy).max(0) as u32;
    }

    fn scroll_to(&mut self, x: u32, y: u32) {
        self.scroll.x = x;
        self.scroll.y = y;
    }

    fn get_scroll(&self) -> (u32, u32) {
        (self.scroll.x, self.scroll.y)
    }

    fn send_key(&mut self, _key: &str) {}

    fn resize(&mut self, _width: u32, _height: u32) {}

    fn render(&mut self) -> Result<()> {
        Ok(())
    }

    fn poll_events(&mut self) -> Vec<EngineEvent> {
        self.events.drain(..).collect()
    }

    fn get_layout_nodes(&self) -> &[LayoutNode] {
        &self.layout_nodes
    }

    fn get_content_height(&self) -> usize {
        self.content_height
    }
}
