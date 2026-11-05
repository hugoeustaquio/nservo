use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOptions {
    pub smooth_scroll: bool,
    pub homepage: String,
    pub adblock_enabled: bool,
    pub user_agent: String,
    pub zoom_level: f32,
}

impl ConfigOptions {
    pub fn new() -> Self {
        Self {
            smooth_scroll: true,
            homepage: "https://servo.org".to_string(),
            adblock_enabled: true,
            user_agent: "NServo/0.1.0 (X11; Linux x86_64) Servo/2026".to_string(),
            zoom_level: 1.0,
        }
    }
}

impl Default for ConfigOptions {
    fn default() -> Self {
        Self::new()
    }
}
