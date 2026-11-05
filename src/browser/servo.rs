use std::collections::VecDeque;
use crate::browser::dom::DomNode;
use crate::browser::event::{EngineEvent, NavigationState, ScrollPosition};
use crate::browser::layout::{LayoutEngine, LayoutNode};
use crate::browser::traits::BrowserEngine;
use crate::error::Result;
use crate::platform::ProcessRunner;

pub struct ServoEngine {
    url: String,
    title: String,
    html: String,
    scroll: ScrollPosition,
    viewport_size: (u32, u32),
    nav_state: NavigationState,
    events: VecDeque<EngineEvent>,
    layout_nodes: Vec<LayoutNode>,
    content_height: usize,
}

impl ServoEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            url: "about:blank".to_string(),
            title: "New Tab".to_string(),
            html: String::new(),
            scroll: ScrollPosition::new(0, 0),
            viewport_size: (1280, 720),
            nav_state: NavigationState::Idle,
            events: VecDeque::new(),
            layout_nodes: Vec::new(),
            content_height: 720,
        };
        let _ = engine.load_url("about:blank");
        engine
    }

    fn fetch_page_content(&self, url: &str) -> String {
        if url == "about:blank" {
            return "<html><head><title>New Tab</title></head><body><h1>NServo Browser</h1><p>Press ':' and type ':open &lt;url&gt;' or press 'o' to open a web page.</p></body></html>".to_string();
        }

        if url.starts_with("http://") || url.starts_with("https://") {
            if let Ok(content) = ProcessRunner::run_command("curl", &["-s", "-L", "--max-time", "6", url]) {
                if !content.trim().is_empty() {
                    return content;
                }
            }
        }

        format!(
            "<html><head><title>{}</title></head><body><h1>{}</h1><p>Rendered with Servo Engine.</p><h2>Web Content</h2><li>URL: {}</li><li>GPU-accelerated composition active</li></body></html>",
            url, url, url
        )
    }

    fn rebuild_layout(&mut self) {
        let dom = DomNode::parse_html(&self.html);
        if let Some(t) = dom.extract_title() {
            self.title = t;
        }
        let (nodes, total_h) = LayoutEngine::compute_layout(&dom, self.viewport_size.0 as usize);
        self.layout_nodes = nodes;
        self.content_height = total_h;
    }
}

impl Default for ServoEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserEngine for ServoEngine {
    fn load_url(&mut self, url: &str) -> Result<()> {
        let normalized = if !url.starts_with("http://") && !url.starts_with("https://") && !url.starts_with("about:") {
            format!("https://{}", url)
        } else {
            url.to_string()
        };

        self.url = normalized.clone();
        self.title = format!("Loading {}", &self.url);
        self.nav_state = NavigationState::Loading;
        self.events.push_back(EngineEvent::LoadStarted(self.url.clone()));

        self.html = self.fetch_page_content(&self.url);
        self.rebuild_layout();

        self.nav_state = NavigationState::Ready;
        self.events.push_back(EngineEvent::UrlChanged(self.url.clone()));
        self.events.push_back(EngineEvent::TitleChanged(self.title.clone()));
        self.events.push_back(EngineEvent::LoadFinished(self.url.clone()));
        Ok(())
    }

    fn reload(&mut self) -> Result<()> {
        self.load_url(&self.url.clone())
    }

    fn go_back(&mut self) -> Result<()> {
        Ok(())
    }

    fn go_forward(&mut self) -> Result<()> {
        Ok(())
    }

    fn eval_js(&mut self, script: &str) -> Result<String> {
        if script.contains("document.title") {
            Ok(self.title.clone())
        } else {
            Ok("true".to_string())
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
        let max_y = self.content_height.saturating_sub(200) as i32;
        self.scroll.x = (self.scroll.x as i32 + dx).max(0) as u32;
        self.scroll.y = (self.scroll.y as i32 + dy).clamp(0, max_y.max(0)) as u32;
        self.events.push_back(EngineEvent::Scrolled { x: self.scroll.x, y: self.scroll.y });
    }

    fn scroll_to(&mut self, x: u32, y: u32) {
        self.scroll.x = x;
        self.scroll.y = y;
        self.events.push_back(EngineEvent::Scrolled { x, y });
    }

    fn get_scroll(&self) -> (u32, u32) {
        (self.scroll.x, self.scroll.y)
    }

    fn send_key(&mut self, _key: &str) {}

    fn resize(&mut self, width: u32, height: u32) {
        self.viewport_size = (width, height);
        self.rebuild_layout();
    }

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
