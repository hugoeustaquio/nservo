pub(crate) struct CommandSpec {
    pub(crate) name: &'static str,
    pub(crate) takes_arg: bool,
    pub(crate) description: &'static str,
}

pub(crate) const COMMAND_SPECS: &[CommandSpec] = &[
    CommandSpec {
        name: "open",
        takes_arg: true,
        description: "Open URL or search in current tab",
    },
    CommandSpec {
        name: "Open",
        takes_arg: true,
        description: "Open URL or search in new tab",
    },
    CommandSpec {
        name: "tabnew",
        takes_arg: false,
        description: "Open a new tab (starter page or URL)",
    },
    CommandSpec {
        name: "tabopen",
        takes_arg: true,
        description: "Open URL or search in new tab",
    },
    CommandSpec {
        name: "tabnext",
        takes_arg: false,
        description: "Switch to next tab",
    },
    CommandSpec {
        name: "tabprev",
        takes_arg: false,
        description: "Switch to previous tab",
    },
    CommandSpec {
        name: "tabclose",
        takes_arg: false,
        description: "Close current tab",
    },
    CommandSpec {
        name: "reload",
        takes_arg: false,
        description: "Reload current tab",
    },
    CommandSpec {
        name: "back",
        takes_arg: false,
        description: "Navigate back in history",
    },
    CommandSpec {
        name: "forward",
        takes_arg: false,
        description: "Navigate forward in history",
    },
    CommandSpec {
        name: "zoom",
        takes_arg: true,
        description: "Set zoom level (0.1–10.0 or reset)",
    },
    CommandSpec {
        name: "inspect",
        takes_arg: false,
        description: "Toggle Web Inspector drawer",
    },
    CommandSpec {
        name: "noh",
        takes_arg: false,
        description: "Clear in-page search highlights",
    },
    CommandSpec {
        name: "w",
        takes_arg: false,
        description: "Save current tabs as session",
    },
    CommandSpec {
        name: "wq",
        takes_arg: false,
        description: "Save session and quit",
    },
    CommandSpec {
        name: "session",
        takes_arg: true,
        description: "Restore a saved session",
    },
    CommandSpec {
        name: "bookmark",
        takes_arg: false,
        description: "Bookmark current page",
    },
    CommandSpec {
        name: "bookmarks",
        takes_arg: false,
        description: "Open bookmarks page",
    },
    CommandSpec {
        name: "bmdel",
        takes_arg: true,
        description: "Remove bookmark by index or URL",
    },
    CommandSpec {
        name: "history",
        takes_arg: false,
        description: "Open or filter browsing history",
    },
    CommandSpec {
        name: "history-clear",
        takes_arg: false,
        description: "Clear all saved history",
    },
    CommandSpec {
        name: "adblock",
        takes_arg: true,
        description: "Download ABP filter list from URL",
    },
    CommandSpec {
        name: "adblock-update",
        takes_arg: false,
        description: "Update saved adblock filter lists",
    },
    CommandSpec {
        name: "plugin",
        takes_arg: true,
        description: "Run an installed plugin",
    },
    CommandSpec {
        name: "plugin-install",
        takes_arg: true,
        description: "Install plugin from GitHub/Git URL",
    },
    CommandSpec {
        name: "plugin-update",
        takes_arg: false,
        description: "Update installed plugins via git pull",
    },
    CommandSpec {
        name: "plugin-remove",
        takes_arg: true,
        description: "Remove an installed plugin",
    },
    CommandSpec {
        name: "config",
        takes_arg: false,
        description: "Open settings & shortcuts (about:config)",
    },
    CommandSpec {
        name: "help",
        takes_arg: false,
        description: "Open keyboard & command reference",
    },
    CommandSpec {
        name: "quit",
        takes_arg: false,
        description: "Quit nservo",
    },
];

/// Compute an fzf-style fuzzy match score for `query` against `candidate`.
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

    total -= (cand_chars.len().min(200) as i32) / 8;
    Some(total)
}

fn score_token(token: &[char], cand: &[char], cand_lower: &str) -> Option<i32> {
    let (mut score, mut pos, mut streak, mut first_match, mut prev_idx): (
        i32,
        usize,
        i32,
        Option<usize>,
        Option<usize>,
    ) = (0, 0, 0, None, None);

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
    matches!(
        prev,
        ' ' | '/' | '.' | '-' | '_' | ':' | '?' | '&' | '=' | '#'
    )
}
