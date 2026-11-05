use crate::bookmarks::BookmarksDatabase;
use crate::command::action::CommandAction;
use crate::error::Result;
use crate::history::HistoryDatabase;
use crate::session::SessionStorage;
use crate::tabs::TabManager;

pub struct ExecutionOutcome {
    pub should_quit: bool,
    pub message: String,
}

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn execute(
        action: CommandAction,
        tabs: &mut TabManager,
        session: &SessionStorage,
        history: &HistoryDatabase,
        bookmarks: &BookmarksDatabase,
    ) -> Result<ExecutionOutcome> {
        match action {
            CommandAction::Open(url) => {
                if let Some(tab) = tabs.current_tab_mut() {
                    tab.load_url(&url)?;
                    let _ = history.record_visit(&tab.url, &tab.title);
                    Ok(ExecutionOutcome {
                        should_quit: false,
                        message: format!("Opened: {}", tab.url),
                    })
                } else {
                    tabs.open_tab(&url)?;
                    Ok(ExecutionOutcome {
                        should_quit: false,
                        message: format!("Opened in new tab: {}", url),
                    })
                }
            }
            CommandAction::OpenNewTab(url) | CommandAction::TabNew(Some(url)) => {
                let idx = tabs.open_tab(&url)?;
                if let Some(tab) = tabs.current_tab() {
                    let _ = history.record_visit(&tab.url, &tab.title);
                }
                Ok(ExecutionOutcome {
                    should_quit: false,
                    message: format!("Opened tab [{}] {}", idx + 1, url),
                })
            }
            CommandAction::TabNew(None) => {
                let idx = tabs.open_tab("about:blank")?;
                Ok(ExecutionOutcome {
                    should_quit: false,
                    message: format!("New blank tab [{}] created", idx + 1),
                })
            }
            CommandAction::TabNext => {
                tabs.next_tab();
                Ok(ExecutionOutcome {
                    should_quit: false,
                    message: format!("Active tab: [{}]", tabs.active_index() + 1),
                })
            }
            CommandAction::TabPrev => {
                tabs.prev_tab();
                Ok(ExecutionOutcome {
                    should_quit: false,
                    message: format!("Active tab: [{}]", tabs.active_index() + 1),
                })
            }
            CommandAction::TabClose => {
                tabs.close_current()?;
                Ok(ExecutionOutcome {
                    should_quit: tabs.tabs().is_empty(),
                    message: "Tab closed".to_string(),
                })
            }
            CommandAction::Tabs => {
                let summary = tabs
                    .tabs()
                    .iter()
                    .enumerate()
                    .map(|(i, t)| format!("[{}]: {}", i + 1, t.title))
                    .collect::<Vec<_>>()
                    .join(", ");
                Ok(ExecutionOutcome {
                    should_quit: false,
                    message: format!("Open tabs: {}", summary),
                })
            }
            CommandAction::WriteQuit => {
                session.save_from_tabs(tabs)?;
                Ok(ExecutionOutcome {
                    should_quit: true,
                    message: "Session saved. Exiting NServo.".to_string(),
                })
            }
            CommandAction::Quit => Ok(ExecutionOutcome {
                should_quit: true,
                message: "Exiting NServo.".to_string(),
            }),
            CommandAction::Bookmark => {
                if let Some(tab) = tabs.current_tab() {
                    bookmarks.add(&tab.url, &tab.title, &[])?;
                    Ok(ExecutionOutcome {
                        should_quit: false,
                        message: format!("Bookmarked: {}", tab.url),
                    })
                } else {
                    Ok(ExecutionOutcome {
                        should_quit: false,
                        message: "No active tab to bookmark".to_string(),
                    })
                }
            }
            CommandAction::Clean => {
                history.clear()?;
                bookmarks.clear()?;
                Ok(ExecutionOutcome {
                    should_quit: false,
                    message: "Clean complete: History and Bookmarks cleared.".to_string(),
                })
            }
            CommandAction::Plugin(name) => Ok(ExecutionOutcome {
                should_quit: false,
                message: format!("Plugin executed: {}", name),
            }),
            CommandAction::Custom(name, _) => Ok(ExecutionOutcome {
                should_quit: false,
                message: format!("Custom command executed: {}", name),
            }),
        }
    }
}
