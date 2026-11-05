use crate::modes::Mode;
use crate::ui::canvas::Canvas;
use crate::ui::colors::GuiColors;

pub struct StatusBarView;

impl StatusBarView {
    pub const HEIGHT: usize = 24;

    pub fn render(
        canvas: &mut Canvas,
        y: usize,
        mode: Mode,
        url: &str,
        title: &str,
        scroll: (u32, u32),
    ) {
        // Status bar background
        canvas.fill_rect(0, y, canvas.width, Self::HEIGHT, GuiColors::STATUS_BG);
        canvas.fill_rect(0, y, canvas.width, 1, GuiColors::BORDER_COLOR);

        // Mode badge
        let (badge_bg, badge_label) = match mode {
            Mode::Normal => (GuiColors::MODE_NORMAL_BG, " NORMAL "),
            Mode::Insert => (GuiColors::MODE_INSERT_BG, " INSERT "),
            Mode::Passthrough => (GuiColors::MODE_PASSTHROUGH_BG, " PASS "),
            Mode::Command => (GuiColors::MODE_COMMAND_BG, " COMMAND "),
        };

        let badge_width = badge_label.len() * 8 + 8;
        canvas.fill_rect(4, y + 2, badge_width, Self::HEIGHT - 4, badge_bg);
        canvas.draw_text(8, y + 8, badge_label, GuiColors::TEXT_WHITE, None);

        // Page title and URL
        let text_start_x = badge_width + 16;
        let info = format!("{} | {}", title, url);
        let max_chars = (canvas.width.saturating_sub(text_start_x + 120)) / 8;

        let display_info = if info.len() > max_chars && max_chars > 3 {
            format!("{}...", &info[..max_chars - 3])
        } else {
            info
        };

        canvas.draw_text(text_start_x, y + 8, &display_info, GuiColors::TEXT_WHITE, None);

        // Scroll coordinate indicator on the right
        let scroll_label = format!("({}, {})", scroll.0, scroll.1);
        let scroll_x = canvas.width.saturating_sub(scroll_label.len() * 8 + 16);
        canvas.draw_text(scroll_x, y + 8, &scroll_label, GuiColors::TEXT_MUTED, None);
    }
}
