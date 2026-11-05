/// Color palette for the graphical UI (Neovim / qutebrowser dark theme).
pub struct GuiColors;

impl GuiColors {
    // 0x00RRGGBB format for softbuffer (ARGB32/XRGB32)
    pub const WINDOW_BG: u32 = 0x0018181b;       // Zinc 900
    pub const VIEWPORT_BG: u32 = 0x001f232a;     // Web content background
    pub const TAB_BAR_BG: u32 = 0x0027272a;      // Zinc 800
    pub const TAB_ACTIVE_BG: u32 = 0x002563eb;   // Blue 600
    pub const TAB_INACTIVE_BG: u32 = 0x003f3f46; // Zinc 700
    pub const TEXT_WHITE: u32 = 0x00ffffff;
    pub const TEXT_MUTED: u32 = 0x00a1a1aa;      // Zinc 400
    pub const TEXT_ACCENT: u32 = 0x0038bdf8;     // Sky 400
    pub const STATUS_BG: u32 = 0x0018181b;
    pub const BORDER_COLOR: u32 = 0x003f3f46;
    pub const CMD_BG: u32 = 0x0009090b;          // Zinc 950
    pub const POPUP_BG: u32 = 0x0027272a;

    // Mode badges
    pub const MODE_NORMAL_BG: u32 = 0x002563eb;      // Blue
    pub const MODE_INSERT_BG: u32 = 0x0016a34a;      // Green
    pub const MODE_PASSTHROUGH_BG: u32 = 0x009333ea; // Purple
    pub const MODE_COMMAND_BG: u32 = 0x00ca8a04;     // Yellow
}
