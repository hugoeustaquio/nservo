use serde::Deserialize;

/// Viewport and document scroll geometry reported by a page's scroll bridge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollMetrics {
    pub y: f64,
    pub view_h: f64,
    pub scroll_h: f64,
    pub x: f64,
    pub view_w: f64,
    pub scroll_w: f64,
}

impl ScrollMetrics {
    pub fn is_v_scrollable(&self) -> bool {
        self.view_h > 0.0 && self.scroll_h > self.view_h + 1.0
    }

    pub fn is_h_scrollable(&self) -> bool {
        self.view_w > 0.0 && self.scroll_w > self.view_w + 1.0
    }

    pub fn max_y(&self) -> f64 {
        (self.scroll_h - self.view_h).max(0.0)
    }

    pub fn max_x(&self) -> f64 {
        (self.scroll_w - self.view_w).max(0.0)
    }
}

#[derive(Deserialize)]
struct RawScrollMetrics {
    #[serde(default)]
    y: f64,
    #[serde(default, rename = "viewH")]
    view_h: f64,
    #[serde(default, rename = "scrollH")]
    scroll_h: f64,
    #[serde(default)]
    x: f64,
    #[serde(default, rename = "viewW")]
    view_w: f64,
    #[serde(default, rename = "scrollW")]
    scroll_w: f64,
}

/// Parse a JSON message emitted by `window.servoGtk.messageHandlers.nservoScroll`.
pub fn parse_scroll_metrics(body: &str) -> Option<ScrollMetrics> {
    let raw: RawScrollMetrics = serde_json::from_str(body).ok()?;
    let values = [
        raw.y,
        raw.view_h,
        raw.scroll_h,
        raw.x,
        raw.view_w,
        raw.scroll_w,
    ];
    if !values.iter().all(|v| v.is_finite() && *v >= 0.0) {
        return None;
    }
    Some(ScrollMetrics {
        y: raw.y,
        view_h: raw.view_h,
        scroll_h: raw.scroll_h.max(raw.view_h),
        x: raw.x,
        view_w: raw.view_w,
        scroll_w: raw.scroll_w.max(raw.view_w),
    })
}
