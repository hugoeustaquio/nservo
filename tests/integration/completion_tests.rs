use nservo::core::{
    Bookmark, CompletionKind, CompletionSources, HistoryEntry, MAX_COMPLETIONS,
    complete_command_input, fuzzy_score,
};

fn sample_sources() -> CompletionSources {
    CompletionSources {
        bookmarks: vec![
            Bookmark {
                url: "https://doc.rust-lang.org/book/".to_string(),
                title: "The Rust Programming Language".to_string(),
            },
            Bookmark {
                url: "https://github.com/servo/servo".to_string(),
                title: "Servo Browser Engine".to_string(),
            },
        ],
        history: vec![
            Bookmark {
                url: "https://github.com/servo/servo".to_string(),
                title: "Servo Browser Engine".to_string(),
            }
            .into_history(1),
            HistoryEntry {
                id: 2,
                url: "https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/".to_string(),
                title: "gtk4 - Rust Crate Documentation".to_string(),
                visited_at: 200,
            },
            HistoryEntry {
                id: 3,
                url: "https://news.ycombinator.com/".to_string(),
                title: "Hacker News".to_string(),
                visited_at: 150,
            },
        ],
        sessions: vec!["default".to_string(), "work".to_string(), "reading".to_string()],
        plugins: Vec::new(),
    }
}

trait IntoHistory {
    fn into_history(self, id: i64) -> HistoryEntry;
}

impl IntoHistory for Bookmark {
    fn into_history(self, id: i64) -> HistoryEntry {
        HistoryEntry {
            id,
            url: self.url,
            title: self.title,
            visited_at: 300,
        }
    }
}

#[test]
fn fuzzy_score_supports_multi_token_fzf_subsequences() {
    assert!(fuzzy_score("", "anything").is_some());
    assert!(fuzzy_score("rsbk", "doc.rust-lang.org/book").is_some());
    assert!(fuzzy_score("gtk doc", "gtk-rs.org gtk4 documentation").is_some());
    assert!(fuzzy_score("zzz", "https://servo.org").is_none());

    // Exact prefix/contiguous match outscores scattered subsequence match.
    let tight = fuzzy_score("servo", "servo.org").unwrap();
    let loose = fuzzy_score("servo", "s-e-r-v-o.example.org").unwrap();
    assert!(tight > loose);
}

#[test]
fn completes_command_names_and_marks_arg_commands_with_trailing_space() {
    let sources = sample_sources();

    let all = complete_command_input("", &sources);
    assert_eq!(all.len(), MAX_COMPLETIONS);
    assert_eq!(all[0].kind, CompletionKind::Command);

    let tab_cmds = complete_command_input("tabn", &sources);
    assert!(!tab_cmds.is_empty());
    assert_eq!(tab_cmds[0].primary, "tabnew");
    assert_eq!(tab_cmds[0].insert_text, "tabnew");

    let open_cmds = complete_command_input("op", &sources);
    assert_eq!(open_cmds[0].primary, "open");
    assert_eq!(open_cmds[0].insert_text, "open ");
}

#[test]
fn completes_urls_fzf_style_across_bookmarks_and_history_without_duplicates() {
    let sources = sample_sources();

    // `:open ` with empty query lists bookmarks first, deduplicated against history.
    let empty_url = complete_command_input("open ", &sources);
    assert_eq!(empty_url.len(), 4);
    assert_eq!(empty_url[0].kind, CompletionKind::Bookmark);
    assert_eq!(empty_url[0].insert_text, "open https://doc.rust-lang.org/book/");
    assert_eq!(
        empty_url
            .iter()
            .filter(|i| i.primary == "https://github.com/servo/servo")
            .count(),
        1,
        "duplicate URL across bookmarks and history must appear once"
    );

    // Multi-token fuzzy query matches both URL and title and preserves `:Open ` prefix.
    let gtk = complete_command_input("Open gtk docs", &sources);
    assert_eq!(gtk.len(), 1);
    assert_eq!(gtk[0].kind, CompletionKind::History);
    assert_eq!(
        gtk[0].insert_text,
        "Open https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/"
    );

    // Works with `:tabnew ` as well and ranks the tightest fuzzy match first.
    let hn = complete_command_input("tabnew hckr", &sources);
    assert!(!hn.is_empty());
    assert_eq!(hn[0].insert_text, "tabnew https://news.ycombinator.com/");
}

#[test]
fn completes_saved_session_names() {
    let sources = sample_sources();
    let matches = complete_command_input("session rd", &sources);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].kind, CompletionKind::Session);
    assert_eq!(matches[0].insert_text, "session reading");
}
