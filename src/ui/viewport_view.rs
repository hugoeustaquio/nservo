use crate::tabs::Tab;
use crate::ui::canvas::Canvas;
use crate::ui::colors::GuiColors;

pub struct ViewportView;

impl ViewportView {
    pub fn render(canvas: &mut Canvas, y: usize, height: usize, tab: Option<&Tab>) {
        // Viewport background
        canvas.fill_rect(0, y, canvas.width, height, GuiColors::VIEWPORT_BG);

        if let Some(tab) = tab {
            let title = tab.engine.get_title();
            let url = tab.engine.get_url();
            let (_sx, sy) = tab.engine.get_scroll();

            // 1. Header card with page title and active URL
            canvas.fill_rect(32, y + 16, canvas.width.saturating_sub(64), 50, 0x002d333b);
            let header = format!("Web Page: {}", title);
            canvas.draw_text(48, y + 26, &header, GuiColors::TEXT_WHITE, None);
            let url_sub = format!("URL: {}  [Engine: Servo WebRender Pipeline]", url);
            canvas.draw_text(48, y + 44, &url_sub, GuiColors::TEXT_ACCENT, None);

            // 2. Dynamic DOM nodes rendered from the engine
            let nodes = tab.engine.get_layout_nodes();
            let content_base_y = y + 80;

            for node in nodes {
                // Apply scroll offset
                let node_y = (node.y + content_base_y).saturating_sub(sy as usize);

                // Cull nodes outside visible viewport
                if node_y + node.height < y + 70 || node_y > y + height {
                    continue;
                }

                if let Some(bg) = node.bg_color {
                    canvas.fill_rect(node.x, node_y, node.width, node.height, bg);
                }

                canvas.draw_text(node.x + 4, node_y + 4, &node.text, node.fg_color, None);
            }

            // 3. Right edge scrollbar
            let content_h = tab.engine.get_content_height().max(height);
            let scrollbar_w = 8;
            let scrollbar_x = canvas.width.saturating_sub(scrollbar_w);
            canvas.fill_rect(scrollbar_x, y, scrollbar_w, height, 0x0027272a);

            let thumb_h = ((height * height) / content_h).clamp(30, height);
            let max_scroll = content_h.saturating_sub(height).max(1);
            let thumb_y = y + ((sy as usize * (height - thumb_h)) / max_scroll);
            canvas.fill_rect(scrollbar_x, thumb_y.min(y + height - thumb_h), scrollbar_w, thumb_h, GuiColors::TAB_ACTIVE_BG);
        } else {
            canvas.draw_text(48, y + 48, "No open tabs. Press 'o' to open a URL.", GuiColors::TEXT_MUTED, None);
        }
    }
}
