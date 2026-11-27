use serde::Deserialize;

/// Events emitted by the injected in-page search script over `nservoSearch`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchEvent {
    Results {
        query: String,
        current: usize,
        total: usize,
        wrapped: bool,
    },
    Cleared,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RawSearchEvent {
    Results {
        query: String,
        current: usize,
        total: usize,
        #[serde(default)]
        wrapped: bool,
    },
    Cleared,
}

/// Parse a JSON message emitted by `window.servoGtk.messageHandlers.nservoSearch`.
pub fn parse_search_message(body: &str) -> Option<SearchEvent> {
    let raw: RawSearchEvent = serde_json::from_str(body).ok()?;
    Some(match raw {
        RawSearchEvent::Results {
            query,
            current,
            total,
            wrapped,
        } => SearchEvent::Results {
            query,
            current,
            total,
            wrapped,
        },
        RawSearchEvent::Cleared => SearchEvent::Cleared,
    })
}

/// Format a Neovim-style statusline message for a search result.
///
/// Returns `(message, is_error)`.
pub fn format_search_status(
    query: &str,
    current: usize,
    total: usize,
    wrapped: bool,
) -> (String, bool) {
    if total == 0 {
        (format!("Pattern not found: {query}"), true)
    } else if wrapped {
        (
            format!("Search wrapped — /{query} [{current}/{total}]"),
            false,
        )
    } else {
        (format!("/{query} [{current}/{total}]"), false)
    }
}

/// Tracks the browser's active search register (similar to Vim's `/` register).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SearchState {
    last_query: Option<String>,
    highlights_active: bool,
}

impl SearchState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve a submitted search prompt.
    ///
    /// A non-empty query becomes the new active search; an empty submission
    /// repeats the last saved query (matching Vim's `/` + `Enter` behavior).
    pub fn submit(&mut self, raw: &str) -> Option<String> {
        let query = raw.trim();
        if !query.is_empty() {
            self.last_query = Some(query.to_string());
            self.highlights_active = true;
            return Some(query.to_string());
        }
        if let Some(prev) = self.last_query.clone() {
            self.highlights_active = true;
            return Some(prev);
        }
        None
    }

    /// Last non-empty search query, used by `n` and `N`.
    pub fn last_query(&self) -> Option<&str> {
        self.last_query.as_deref()
    }

    /// Whether page highlights are currently shown.
    pub fn has_active_highlights(&self) -> bool {
        self.highlights_active
    }

    /// Mark highlights as active (for example when stepping with `n`/`N`).
    pub fn mark_active(&mut self) {
        if self.last_query.is_some() {
            self.highlights_active = true;
        }
    }

    /// Clear active highlight state while keeping the last query for `n`/`N`.
    pub fn clear_highlights(&mut self) -> bool {
        std::mem::replace(&mut self.highlights_active, false)
    }
}
