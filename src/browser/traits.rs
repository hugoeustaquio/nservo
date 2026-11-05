use crate::browser::event::EngineEvent;
use crate::browser::layout::LayoutNode;
use crate::error::Result;

/// Core interface for browser rendering backends (Servo, WebRender, wgpu, headless).
pub trait BrowserEngine: Send + Sync {
    /// Navigate to the given URL.
    fn load_url(&mut self, url: &str) -> Result<()>;

    /// Reload the current page.
    fn reload(&mut self) -> Result<()>;

    /// Navigate back in session history.
    fn go_back(&mut self) -> Result<()>;

    /// Navigate forward in session history.
    fn go_forward(&mut self) -> Result<()>;

    /// Execute a JavaScript snippet and return result.
    fn eval_js(&mut self, script: &str) -> Result<String>;

    /// Retrieve inner or full HTML of the active page.
    fn get_html(&self) -> String;

    /// Retrieve current page title.
    fn get_title(&self) -> String;

    /// Retrieve current page URL.
    fn get_url(&self) -> String;

    /// Scroll by a relative delta (dx, dy).
    fn scroll_by(&mut self, dx: i32, dy: i32);

    /// Scroll to an absolute coordinate (x, y).
    fn scroll_to(&mut self, x: u32, y: u32);

    /// Get current scroll coordinates (x, y).
    fn get_scroll(&self) -> (u32, u32);

    /// Send a key stroke into page DOM inputs.
    fn send_key(&mut self, key: &str);

    /// Update rendering viewport dimensions.
    fn resize(&mut self, width: u32, height: u32);

    /// Perform a render/composition tick.
    fn render(&mut self) -> Result<()>;

    /// Poll pending engine events.
    fn poll_events(&mut self) -> Vec<EngineEvent>;

    /// Rendered layout nodes ready for viewport presentation.
    fn get_layout_nodes(&self) -> &[LayoutNode];

    /// Total height of the parsed document in pixels.
    fn get_content_height(&self) -> usize;
}
