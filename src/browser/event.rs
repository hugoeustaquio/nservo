/// Events dispatched from or to the web rendering engine.
#[derive(Debug, Clone, PartialEq)]
pub enum EngineEvent {
    TitleChanged(String),
    UrlChanged(String),
    LoadStarted(String),
    LoadFinished(String),
    LoadFailed { url: String, error: String },
    Scrolled { x: u32, y: u32 },
}

/// Navigation state of a browser page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavigationState {
    #[default]
    Idle,
    Loading,
    Ready,
    Error,
}

/// Position of viewport scroll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScrollPosition {
    pub x: u32,
    pub y: u32,
}

impl ScrollPosition {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}
