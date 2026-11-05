use crate::ui::canvas::Canvas;
use crate::ui::colors::GuiColors;

pub struct CommandBarView;

impl CommandBarView {
    pub const HEIGHT: usize = 24;

    pub fn render(
        canvas: &mut Canvas,
        y: usize,
        cmd_input: &str,
        suggestions: &[String],
        status_msg: Option<&str>,
        is_command_mode: bool,
    ) {
        // 1. Render Autocomplete popup floating above command bar
        if is_command_mode && !suggestions.is_empty() {
            let max_items = suggestions.len().min(5);
            let item_height = 20;
            let popup_h = max_items * item_height + 4;
            let popup_y = y.saturating_sub(popup_h);
            let popup_w = (canvas.width / 2).max(360).min(canvas.width);

            // Shadow / background
            canvas.fill_rect(0, popup_y, popup_w, popup_h, GuiColors::POPUP_BG);
            canvas.fill_rect(0, popup_y, popup_w, 1, GuiColors::BORDER_COLOR);

            for (i, sug) in suggestions.iter().take(max_items).enumerate() {
                let item_y = popup_y + 2 + i * item_height;
                let text = format!(" > {}", sug);
                canvas.draw_text(8, item_y + 6, &text, GuiColors::TEXT_ACCENT, None);
            }
        }

        // 2. Command bar line background
        canvas.fill_rect(0, y, canvas.width, Self::HEIGHT, GuiColors::CMD_BG);

        if is_command_mode {
            let prompt = format!(":{}", cmd_input);
            canvas.draw_text(8, y + 8, &prompt, GuiColors::TEXT_WHITE, None);

            // Draw graphical blinking block cursor
            let cursor_x = 8 + prompt.len() * 8;
            canvas.fill_rect(cursor_x, y + 6, 8, 12, GuiColors::TEXT_ACCENT);
        } else if let Some(msg) = status_msg {
            canvas.draw_text(8, y + 8, msg, GuiColors::TEXT_MUTED, None);
        }
    }
}
