/// Android platform bindings and native activity abstraction.
///
/// On Android, NServo reuses the entire Rust core (tabs, adblock, Lua plugins,
/// Greasemonkey, and modes). Input events from physical keyboards or on-screen
/// keyboards are dispatched to the mode state machine.

#[derive(Debug, Clone, Default)]
pub struct AndroidContext {
    pub package_name: String,
    pub files_dir: String,
    pub is_physical_keyboard: bool,
}

impl AndroidContext {
    pub fn new(package_name: &str, files_dir: &str) -> Self {
        Self {
            package_name: package_name.to_string(),
            files_dir: files_dir.to_string(),
            is_physical_keyboard: false,
        }
    }

    pub fn set_physical_keyboard(&mut self, enabled: bool) {
        self.is_physical_keyboard = enabled;
    }
}

/// Helper to map Android keycodes to NServo key actions.
pub fn map_android_keycode(keycode: i32) -> Option<String> {
    match keycode {
        19 => Some("k".to_string()),    // DPAD_UP
        20 => Some("j".to_string()),    // DPAD_DOWN
        21 => Some("h".to_string()),    // DPAD_LEFT
        22 => Some("l".to_string()),    // DPAD_RIGHT
        111 => Some("ESC".to_string()), // KEYCODE_ESCAPE
        66 => Some("ENTER".to_string()),// KEYCODE_ENTER
        _ => None,
    }
}
