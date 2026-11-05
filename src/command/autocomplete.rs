use crate::bookmarks::BookmarksDatabase;
use crate::history::HistoryDatabase;

pub struct AutocompleteEngine;

impl AutocompleteEngine {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &[
        "open", "Open", "tabnew", "tabnext", "tabprev", "tabclose",
        "tabs", "wq", "q", "plugin", "bookmark", "clean"
    ];

    pub fn suggest(
        input: &str,
        history: Option<&HistoryDatabase>,
        bookmarks: Option<&BookmarksDatabase>,
    ) -> Vec<String> {
        let clean = input.trim().trim_start_matches(':');
        if clean.is_empty() {
            return Vec::new();
        }

        let mut suggestions = Vec::new();

        // 1. Command completions
        for cmd in Self::BUILTIN_COMMANDS {
            if cmd.starts_with(clean) {
                suggestions.push(format!(":{}", cmd));
            }
        }

        // 2. Bookmarks completions
        if let Some(bm_db) = bookmarks {
            if let Ok(matches) = bm_db.search(clean) {
                for b in matches {
                    if !suggestions.contains(&b.url) {
                        suggestions.push(b.url);
                    }
                }
            }
        }

        // 3. History completions (frequency and recency weighted)
        if let Some(hist_db) = history {
            if let Ok(matches) = hist_db.search(clean, 10) {
                for h in matches {
                    if !suggestions.contains(&h.url) {
                        suggestions.push(h.url);
                    }
                }
            }
        }

        suggestions
    }
}
