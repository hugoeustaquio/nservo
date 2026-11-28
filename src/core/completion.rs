use std::collections::HashSet;

use crate::core::completion_specs::COMMAND_SPECS;
pub use crate::core::completion_specs::fuzzy_score;
use crate::core::{Bookmark, HistoryEntry};

/// Maximum number of suggestions returned for the command-bar completion list.
pub const MAX_COMPLETIONS: usize = 10;

/// Category badge shown next to each completion suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Command,
    Bookmark,
    History,
    Session,
    Plugin,
}

impl CompletionKind {
    pub const fn badge(self) -> &'static str {
        match self {
            Self::Command => "CMD",
            Self::Bookmark => "BM",
            Self::History => "HIST",
            Self::Session => "SESS",
            Self::Plugin => "PLUG",
        }
    }
}

/// Single completion suggestion rendered in the fzf-style command popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub kind: CompletionKind,
    pub primary: String,
    pub secondary: String,
    pub insert_text: String,
}

/// Snapshot of user data queried by the pure completion engine.
#[derive(Debug, Default, Clone)]
pub struct CompletionSources {
    pub bookmarks: Vec<Bookmark>,
    pub history: Vec<HistoryEntry>,
    pub sessions: Vec<String>,
    pub plugins: Vec<(String, String)>,
}

/// Produce ranked fzf-style completions for the current `:` command-bar `input`.
pub fn complete_command_input(input: &str, sources: &CompletionSources) -> Vec<CompletionItem> {
    let trimmed_start = input.strip_prefix(':').unwrap_or(input).trim_start();
    if let Some((cmd, rest)) = split_command_and_arg(trimmed_start) {
        return match cmd {
            "open" | "o" | "Open" | "O" | "tabopen" | "tabnew" => complete_urls(cmd, rest, sources),
            "session" | "w" | "write" | "wq" => complete_sessions(cmd, rest, &sources.sessions),
            "plugin" | "plug" | "plugin-update" | "plugin-remove" => {
                complete_plugins(cmd, rest, &sources.plugins)
            }
            _ => Vec::new(),
        };
    }
    complete_commands(trimmed_start)
}

fn split_command_and_arg(text: &str) -> Option<(&str, &str)> {
    let idx = text.find(char::is_whitespace)?;
    let cmd = &text[..idx];
    (!cmd.is_empty()).then_some((cmd, text[idx..].trim_start()))
}

fn complete_commands(query: &str) -> Vec<CompletionItem> {
    let mut scored = Vec::new();
    for (order, spec) in COMMAND_SPECS.iter().enumerate() {
        let score = if let Some(mut name_score) = fuzzy_score(query, spec.name) {
            name_score += 80;
            if !query.is_empty() && spec.name.starts_with(query) {
                name_score += 60;
            } else if !query.is_empty()
                && spec
                    .name
                    .to_ascii_lowercase()
                    .starts_with(&query.to_ascii_lowercase())
            {
                name_score += 40;
            }
            name_score
        } else if let Some(desc_score) = fuzzy_score(query, spec.description) {
            desc_score
        } else {
            continue;
        };
        let insert_text = if spec.takes_arg {
            format!("{} ", spec.name)
        } else {
            spec.name.to_string()
        };
        scored.push((
            score,
            order,
            CompletionItem {
                kind: CompletionKind::Command,
                primary: spec.name.to_string(),
                secondary: spec.description.to_string(),
                insert_text,
            },
        ));
    }
    sort_and_take(scored)
}

fn score_url_entry(query: &str, url: &str, title: &str) -> Option<i32> {
    let clean_url = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    if let Some(score) = [fuzzy_score(query, clean_url), fuzzy_score(query, title)]
        .into_iter()
        .flatten()
        .max()
    {
        return Some(score + 20);
    }
    fuzzy_score(query, &format!("{clean_url} {title}"))
}

fn complete_urls(cmd: &str, query: &str, sources: &CompletionSources) -> Vec<CompletionItem> {
    let mut seen = HashSet::new();
    let mut scored = Vec::new();
    let mut order = 0usize;

    for bm in &sources.bookmarks {
        if seen.insert(bm.url.clone())
            && let Some(score) = score_url_entry(query, &bm.url, &bm.title)
        {
            scored.push((
                score + 18,
                order,
                CompletionItem {
                    kind: CompletionKind::Bookmark,
                    primary: bm.url.clone(),
                    secondary: bm.title.clone(),
                    insert_text: format!("{cmd} {}", bm.url),
                },
            ));
            order += 1;
        }
    }
    for entry in &sources.history {
        if seen.insert(entry.url.clone())
            && let Some(score) = score_url_entry(query, &entry.url, &entry.title)
        {
            scored.push((
                score,
                order,
                CompletionItem {
                    kind: CompletionKind::History,
                    primary: entry.url.clone(),
                    secondary: entry.title.clone(),
                    insert_text: format!("{cmd} {}", entry.url),
                },
            ));
            order += 1;
        }
    }
    sort_and_take(scored)
}

fn complete_sessions(cmd: &str, query: &str, sessions: &[String]) -> Vec<CompletionItem> {
    let mut scored = Vec::new();
    for (order, name) in sessions.iter().enumerate() {
        if let Some(score) = fuzzy_score(query, name) {
            scored.push((
                score,
                order,
                CompletionItem {
                    kind: CompletionKind::Session,
                    primary: name.clone(),
                    secondary: "Saved session".to_string(),
                    insert_text: format!("{cmd} {name}"),
                },
            ));
        }
    }
    sort_and_take(scored)
}

fn complete_plugins(cmd: &str, query: &str, plugins: &[(String, String)]) -> Vec<CompletionItem> {
    let mut scored = Vec::new();
    for (order, (name, desc)) in plugins.iter().enumerate() {
        if let Some(score) = fuzzy_score(query, name).or_else(|| fuzzy_score(query, desc)) {
            scored.push((
                score,
                order,
                CompletionItem {
                    kind: CompletionKind::Plugin,
                    primary: name.clone(),
                    secondary: desc.clone(),
                    insert_text: format!("{cmd} {name}"),
                },
            ));
        }
    }
    sort_and_take(scored)
}

fn sort_and_take(mut scored: Vec<(i32, usize, CompletionItem)>) -> Vec<CompletionItem> {
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    scored
        .into_iter()
        .take(MAX_COMPLETIONS)
        .map(|(_, _, item)| item)
        .collect()
}
