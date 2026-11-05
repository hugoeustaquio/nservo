use crate::command::action::CommandAction;
use crate::error::{NServoError, Result};

pub struct CommandParser;

impl CommandParser {
    pub fn parse(line: &str) -> Result<CommandAction> {
        let trimmed = line.trim().trim_start_matches(':');
        if trimmed.is_empty() {
            return Err(NServoError::CommandSyntax("Empty command line".to_string()));
        }

        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let remainder = parts.collect::<Vec<&str>>().join(" ");

        match cmd {
            "open" => {
                if remainder.is_empty() {
                    Err(NServoError::CommandSyntax("Usage: :open <url>".to_string()))
                } else {
                    Ok(CommandAction::Open(remainder))
                }
            }
            "Open" => {
                if remainder.is_empty() {
                    Err(NServoError::CommandSyntax("Usage: :Open <url>".to_string()))
                } else {
                    Ok(CommandAction::OpenNewTab(remainder))
                }
            }
            "tabnew" => {
                let url = if remainder.is_empty() {
                    None
                } else {
                    Some(remainder)
                };
                Ok(CommandAction::TabNew(url))
            }
            "tabnext" => Ok(CommandAction::TabNext),
            "tabprev" => Ok(CommandAction::TabPrev),
            "tabclose" => Ok(CommandAction::TabClose),
            "tabs" => Ok(CommandAction::Tabs),
            "wq" => Ok(CommandAction::WriteQuit),
            "q" => Ok(CommandAction::Quit),
            "plugin" => {
                if remainder.is_empty() {
                    Err(NServoError::CommandSyntax("Usage: :plugin <name>".to_string()))
                } else {
                    Ok(CommandAction::Plugin(remainder))
                }
            }
            "bookmark" => Ok(CommandAction::Bookmark),
            "clean" => Ok(CommandAction::Clean),
            other => {
                let args = remainder.split_whitespace().map(String::from).collect();
                Ok(CommandAction::Custom(other.to_string(), args))
            }
        }
    }
}
