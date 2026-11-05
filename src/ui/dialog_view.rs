use crate::ui::canvas::Canvas;
use crate::ui::colors::GuiColors;

pub struct DialogView;

impl DialogView {
    pub fn render_confirm_modal(canvas: &mut Canvas, question: &str) {
        let modal_w = 480.min(canvas.width.saturating_sub(32));
        let modal_h = 140.min(canvas.height.saturating_sub(32));
        let modal_x = (canvas.width.saturating_sub(modal_w)) / 2;
        let modal_y = (canvas.height.saturating_sub(modal_h)) / 2;

        // Modal backdrop and border
        canvas.fill_rect(modal_x, modal_y, modal_w, modal_h, GuiColors::POPUP_BG);
        canvas.fill_rect(modal_x, modal_y, modal_w, 2, GuiColors::MODE_COMMAND_BG);

        // Header
        canvas.draw_text(modal_x + 16, modal_y + 16, "[CONFIRMATION REQUIRED]", GuiColors::MODE_COMMAND_BG, None);

        // Question text
        canvas.draw_text(modal_x + 16, modal_y + 48, question, GuiColors::TEXT_WHITE, None);

        // Buttons
        canvas.fill_rect(modal_x + 16, modal_y + 90, 80, 24, GuiColors::MODE_NORMAL_BG);
        canvas.draw_text(modal_x + 36, modal_y + 98, "[Y] Yes", GuiColors::TEXT_WHITE, None);

        canvas.fill_rect(modal_x + 110, modal_y + 90, 80, 24, GuiColors::TAB_INACTIVE_BG);
        canvas.draw_text(modal_x + 132, modal_y + 98, "[N] No", GuiColors::TEXT_MUTED, None);
    }
}
