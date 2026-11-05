use crate::tabs::TabManager;
use crate::ui::canvas::Canvas;
use crate::ui::colors::GuiColors;

pub struct TabsView;

impl TabsView {
    pub const HEIGHT: usize = 28;

    pub fn render(canvas: &mut Canvas, tabs: &TabManager) {
        // Render tab bar background
        canvas.fill_rect(0, 0, canvas.width, Self::HEIGHT, GuiColors::TAB_BAR_BG);

        let mut x = 4;
        for (i, tab) in tabs.tabs().iter().enumerate() {
            let is_active = i == tabs.active_index();
            let bg_color = if is_active {
                GuiColors::TAB_ACTIVE_BG
            } else {
                GuiColors::TAB_INACTIVE_BG
            };

            let title = if tab.title.len() > 18 {
                format!("{}...", &tab.title[..15])
            } else {
                tab.title.clone()
            };

            let tab_label = format!(" {}: {} ", i + 1, title);
            let tab_width = tab_label.len() * 8 + 12;

            if x + tab_width > canvas.width {
                break;
            }

            // Draw tab capsule
            canvas.fill_rect(x, 2, tab_width, Self::HEIGHT - 4, bg_color);
            let text_color = if is_active {
                GuiColors::TEXT_WHITE
            } else {
                GuiColors::TEXT_MUTED
            };

            canvas.draw_text(x + 6, 10, &tab_label, text_color, None);
            x += tab_width + 4;
        }

        // Bottom separator border
        canvas.fill_rect(0, Self::HEIGHT - 1, canvas.width, 1, GuiColors::BORDER_COLOR);
    }
}
