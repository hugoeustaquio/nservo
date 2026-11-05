use crate::app::state::App;
use crate::command::{CommandExecutor, CommandParser};
use crate::error::Result;
use crate::modes::Mode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuiKey {
    Char(char),
    Enter,
    Escape,
    Backspace,
    Ctrl(char),
    Other,
}

pub struct AppEventHandler;

impl AppEventHandler {
    pub fn handle_key(app: &mut App, key: GuiKey) -> Result<()> {
        match app.modes.current() {
            Mode::Normal => Self::handle_normal_mode(app, key),
            Mode::Insert => Self::handle_insert_mode(app, key),
            Mode::Passthrough => Self::handle_passthrough_mode(app, key),
            Mode::Command => Self::handle_command_mode(app, key),
        }
    }

    fn handle_normal_mode(app: &mut App, key: GuiKey) -> Result<()> {
        match key {
            GuiKey::Char('j') => {
                if let Some(tab) = app.tabs.current_tab_mut() {
                    tab.engine.scroll_by(0, 50);
                }
            }
            GuiKey::Char('k') => {
                if let Some(tab) = app.tabs.current_tab_mut() {
                    tab.engine.scroll_by(0, -50);
                }
            }
            GuiKey::Char('h') => {
                if let Some(tab) = app.tabs.current_tab_mut() {
                    tab.engine.scroll_by(-50, 0);
                }
            }
            GuiKey::Char('l') => {
                if let Some(tab) = app.tabs.current_tab_mut() {
                    tab.engine.scroll_by(50, 0);
                }
            }
            GuiKey::Char('d') => {
                app.tabs.close_current()?;
                if app.tabs.tabs().is_empty() {
                    app.should_quit = true;
                }
            }
            GuiKey::Char('i') => {
                app.modes.transition_to(Mode::Insert);
            }
            GuiKey::Char(':') => {
                app.cmd_buffer.clear();
                app.suggestions.clear();
                app.modes.transition_to(Mode::Command);
            }
            GuiKey::Char('o') => {
                app.cmd_buffer = "open ".to_string();
                app.modes.transition_to(Mode::Command);
            }
            GuiKey::Char('O') => {
                app.cmd_buffer = "Open ".to_string();
                app.modes.transition_to(Mode::Command);
            }
            GuiKey::Ctrl('v') => {
                app.modes.transition_to(Mode::Passthrough);
            }
            GuiKey::Ctrl('c') => {
                app.should_quit = true;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_insert_mode(app: &mut App, key: GuiKey) -> Result<()> {
        match key {
            GuiKey::Escape => {
                app.modes.transition_to(Mode::Normal);
            }
            GuiKey::Char(c) => {
                if let Some(tab) = app.tabs.current_tab_mut() {
                    tab.engine.send_key(&c.to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_passthrough_mode(app: &mut App, key: GuiKey) -> Result<()> {
        match key {
            GuiKey::Escape => {
                app.modes.transition_to(Mode::Normal);
            }
            GuiKey::Char(c) => {
                if let Some(tab) = app.tabs.current_tab_mut() {
                    tab.engine.send_key(&c.to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_command_mode(app: &mut App, key: GuiKey) -> Result<()> {
        match key {
            GuiKey::Escape => {
                app.cmd_buffer.clear();
                app.suggestions.clear();
                app.modes.transition_to(Mode::Normal);
            }
            GuiKey::Enter => {
                let cmd_str = app.cmd_buffer.clone();
                app.cmd_buffer.clear();
                app.suggestions.clear();
                app.modes.transition_to(Mode::Normal);

                if let Ok(action) = CommandParser::parse(&cmd_str) {
                    let hist = app.history.lock().unwrap();
                    let bm = app.bookmarks.lock().unwrap();
                    let outcome = CommandExecutor::execute(
                        action,
                        &mut app.tabs,
                        &app.session,
                        &hist,
                        &bm,
                    )?;
                    app.status_message = Some(outcome.message);
                    if outcome.should_quit {
                        app.should_quit = true;
                    }
                } else {
                    app.status_message = Some(format!("Invalid command: {}", cmd_str));
                }
            }
            GuiKey::Backspace => {
                app.cmd_buffer.pop();
                app.update_suggestions();
            }
            GuiKey::Char(c) => {
                app.cmd_buffer.push(c);
                app.update_suggestions();
            }
            _ => {}
        }
        Ok(())
    }
}
