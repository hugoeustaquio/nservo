use crate::modes::Mode;
use crate::tabs::TabManager;
use crate::ui::canvas::Canvas;
use crate::ui::command_bar_view::CommandBarView;
use crate::ui::dialog_view::DialogView;
use crate::ui::status_bar_view::StatusBarView;
use crate::ui::tabs_view::TabsView;
use crate::ui::viewport_view::ViewportView;

pub struct GuiRenderer;

impl GuiRenderer {
    pub fn render_frame(
        canvas: &mut Canvas,
        tabs: &TabManager,
        mode: Mode,
        cmd_input: &str,
        suggestions: &[String],
        status_msg: Option<&str>,
        confirm_question: Option<&str>,
    ) {
        let tabs_h = TabsView::HEIGHT;
        let status_h = StatusBarView::HEIGHT;
        let cmd_h = CommandBarView::HEIGHT;

        let total_bottom_h = status_h + cmd_h;
        let viewport_h = canvas.height.saturating_sub(tabs_h + total_bottom_h);
        let status_y = canvas.height.saturating_sub(total_bottom_h);
        let cmd_y = canvas.height.saturating_sub(cmd_h);

        // 1. Top Tab Bar
        TabsView::render(canvas, tabs);

        // 2. Main Web Viewport
        ViewportView::render(canvas, tabs_h, viewport_h, tabs.current_tab());

        // 3. Status Bar
        let (url, title, scroll) = if let Some(cur) = tabs.current_tab() {
            (cur.url.clone(), cur.title.clone(), cur.get_scroll())
        } else {
            ("about:blank".to_string(), "NServo".to_string(), (0, 0))
        };
        StatusBarView::render(canvas, status_y, mode, &url, &title, scroll);

        // 4. Command Bar / Autocomplete
        CommandBarView::render(
            canvas,
            cmd_y,
            cmd_input,
            suggestions,
            status_msg,
            mode == Mode::Command,
        );

        // 5. Confirmation Modal (if active)
        if let Some(question) = confirm_question {
            DialogView::render_confirm_modal(canvas, question);
        }
    }
}
