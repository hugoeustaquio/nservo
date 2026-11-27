use std::collections::HashSet;

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
}

impl CompletionKind {
    pub const fn badge(self) -> &'static str {
        match self {
            Self::Command => "CMD",
            Self::Bookmark => "BM",
            Self::History => "HIST",
            Self::Session => "SESS",
        }
    }
}

/// Single completion suggestion rendered in the fzf-style command popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub kind: CompletionKind,
    /// Primary text shown in the row (command name, URL, or session name).
    pub primary: String,
    /// Secondary label shown beside the primary text (description or page title).
    pub secondary: String,
    /// Full command-bar text inserted when this item is selected.
    pub insert_text: String,
}

/// Snapshot of user data queried by the pure completion engine.
#[derive(Debug, Default, Clone)]
pub struct CompletionSources {
    pub bookmarks: Vec<Bookmark>,
    pub history: Vec<HistoryEntry>,
    pub sessions: Vec<String>,
}

struct CommandSpec {
    name: &'static str,
    takes_arg: bool,
    description: &'static str,
}

const COMMAND_SPECS: &[CommandSpec] = &[
    CommandSpec { name: "open", takes_arg: true, description: "Open URL or search in current tab" },
    CommandSpec { name: "Open", takes_arg: true, description: "Open URL or search in new tab" },
    CommandSpec { name: "tabnew", takes_arg: false, description: "Open a new tab (starter page or URL)" },
    CommandSpec { name: "tabopen", takes_arg: true, description: "Open URL or search in new tab" },
    CommandSpec { name: "tabnext", takes_arg: false, description: "Switch to next tab" },
    CommandSpec { name: "tabprev", takes_arg: false, description: "Switch to previous tab" },
    CommandSpec { name: "tabclose", takes_arg: false, description: "Close current tab" },
    CommandSpec { name: "reload", takes_arg: false, description: "Reload current tab" },
    CommandSpec { name: "back", takes_arg: false, description: "Navigate back in history" },
    CommandSpec { name: "forward", takes_arg: false, description: "Navigate forward in history" },
    CommandSpec { name: "zoom", takes_arg: true, description: "Set zoom level (0.1–10.0 or reset)" },
    CommandSpec { name: "inspect", takes_arg: false, description: "Toggle Web Inspector drawer" },
    CommandSpec { name: "noh", takes_arg: false, description: "Clear in-page search highlights" },
    CommandSpec { name: "w", takes_arg: false, description: "Save current tabs as session" },
    CommandSpec { name: "wq", takes_arg: false, description: "Save session and quit" },
    CommandSpec { name: "session", takes_arg: true, description: "Restore a saved session" },
    CommandSpec { name: "bookmark", takes_arg: false, description: "Bookmark current page" },
    CommandSpec { name: "bookmarks", takes_arg: false, description: "Open bookmarks page" },
    CommandSpec { name: "bmdel", takes_arg: true, description: "Remove bookmark by index or URL" },
    CommandSpec { name: "history", takes_arg: false, description: "Open or filter browsing history" },
    CommandSpec { name: "history-clear", takes_arg: false, description: "Clear all saved history" },
    CommandSpec { name: "help", takes_arg: false, description: "Open keyboard & command reference" },
    CommandSpec { name: "quit", takes_arg: false, description: "Quit nservo" },
];

/// Compute an fzf-style fuzzy match score for `query` against `candidate`.
///
/// Splits `query` on whitespace; every token must match `candidate` as a
/// case-insensitive subsequence. Higher scores indicate tighter, earlier, and
/// word-boundary-aligned matches. Returns `None` if any token fails to match.
pub fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Some(0);
    }
    let cand_chars: Vec<char> = candidate.chars().map(|c| c.to_ascii_lowercase()).collect();
    let cand_lower: String = cand_chars.iter().collect();

    let mut total = 0i32;
    for token in trimmed.split_whitespace() {
        let tok_lower: Vec<char> = token.chars().map(|c| c.to_ascii_lowercase()).collect();
        total += score_token(&tok_lower, &cand_chars, &cand_lower)?;
    }

    // Mild preference for shorter candidates on equal match quality.
    total -= (cand_chars.len().min(200) as i32) / 8;
    Some(total)
}

fn score_token(token: &[char], cand: &[char], cand_lower: &str) -> Option<i32> {
    let mut score = 0i32;
    let mut pos = 0usize;
    let mut streak = 0i32;
    let mut first_match = None;
    let mut prev_idx: Option<usize> = None;

    for &needle in token {
        let mut found = None;
        while pos < cand.len() {
            if cand[pos] == needle {
                found = Some(pos);
                pos += 1;
                break;
            }
            pos += 1;
        }
        let idx = found?;
        if first_match.is_none() {
            first_match = Some(idx);
        }
        if let Some(prev) = prev_idx {
            if idx == prev + 1 {
                streak += 1;
            } else {
                streak = 1;
                score -= ((idx - prev - 1).min(24)) as i32;
            }
        } else {
            streak = 1;
        }
        prev_idx = Some(idx);
        score += 12 + streak * 5;
        if idx == 0 || is_boundary(cand[idx - 1]) {
            score += 14;
        }
    }

    let token_str: String = token.iter().collect();
    if cand_lower.starts_with(&token_str) {
        score += 35;
    } else if cand_lower.contains(&token_str) {
        score += 20;
    }
    if let Some(start) = first_match {
        score -= (start.min(80) as i32) / 3;
    }
    Some(score)
}

fn is_boundary(prev: char) -> bool {
    matches!(prev, ' ' | '/' | '.' | '-' | '_' | ':' | '?' | '&' | '=' | '#')
}

/// Produce ranked fzf-style completions for the current `:` command-bar `input`.
pub fn complete_command_input(input: &str, sources: &CompletionSources) -> Vec<CompletionItem> {
    let raw = input.strip_prefix(':').unwrap_or(input);
    let trimmed_start = raw.trim_start();

    if let Some((cmd, rest)) = split_command_and_arg(trimmed_start) {
        return match cmd {
            "open" | "o" | "Open" | "O" | "tabopen" | "tabnew" => {
                complete_urls(cmd, rest, sources)
            }
            "session" | "w" | "write" | "wq" => complete_sessions(cmd, rest, &sources.sessions),
            _ => Vec::new(),
        };
    }

    complete_commands(trimmed_start)
}

fn split_command_and_arg(text: &str) -> Option<(&str, &str)> {
    let idx = text.find(char::is_whitespace)?;
    let cmd = &text[..idx];
    let rest = text[idx..].trim_start();
    (!cmd.is_empty()).then_some((cmd, rest))
}

fn complete_commands(query: &str) -> Vec<CompletionItem> {
    let mut scored: Vec<(i32, usize, CompletionItem)> = Vec::new();
    for (order, spec) in COMMAND_SPECS.iter().enumerate() {
        let score = if let Some(mut name_score) = fuzzy_score(query, spec.name) {
            name_score += 80;
            if !query.is_empty() && spec.name.starts_with(query) {
                name_score += 60;
            } else if !query.is_empty() && spec.name.to_ascii_lowercase().starts_with(&query.to_ascii_lowercase()) {
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
    let best_single = [fuzzy_score(query, clean_url), fuzzy_score(query, title)]
        .into_iter()
        .flatten()
        .max();
    if let Some(score) = best_single {
        return Some(score + 20);
    }
    let combined = format!("{clean_url} {title}");
    fuzzy_score(query, &combined)
}

fn complete_urls(cmd: &str, query: &str, sources: &CompletionSources) -> Vec<CompletionItem> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut scored: Vec<(i32, usize, CompletionItem)> = Vec::new();
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
    let mut scored: Vec<(i32, usize, CompletionItem)> = Vec::new();
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

fn sort_and_take(mut scored: Vec<(i32, usize, CompletionItem)>) -> Vec<CompletionItem> {
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    scored.into_iter().take(MAX_COMPLETIONS).map(|(_, _, item)| item).collect()
}
