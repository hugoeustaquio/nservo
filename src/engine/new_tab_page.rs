use std::fmt::Write;
use std::fs;
use std::path::Path;

use crate::core::{Bookmark, BookmarkStore, HistoryEntry, HistoryStore, default_startpage_path};

/// URL assigned to the built-in new-tab starter page (`:tabnew`).
pub const START_PAGE_URL: &str = "about:start";

/// Default search engine endpoint submitted by the starter page search bar.
pub const DEFAULT_SEARCH_ENGINE_ACTION: &str = "https://www.google.com/search";

const START_PAGE_BOOKMARKS_LIMIT: usize = 50;
const START_PAGE_HISTORY_LIMIT: usize = 30;

/// Load the start page HTML for `:tabnew`: uses `$XDG_CONFIG_HOME/nservo/startpage.html`
/// when present and readable, otherwise renders the default starter page with recent
/// bookmarks and navigation history from SQLite.
pub fn load_start_page_html() -> String {
    let custom_path = default_startpage_path();
    let bookmarks = BookmarkStore::default_store()
        .list_recent(START_PAGE_BOOKMARKS_LIMIT)
        .unwrap_or_default();
    let history = HistoryStore::default_store()
        .recent(START_PAGE_HISTORY_LIMIT)
        .unwrap_or_default();
    resolve_start_page_html(&custom_path, &bookmarks, &history)
}

/// Resolve start page HTML from `custom_path` if it exists as a readable file,
/// falling back to the built-in default starter page.
pub fn resolve_start_page_html(
    custom_path: &Path,
    bookmarks: &[Bookmark],
    history: &[HistoryEntry],
) -> String {
    if custom_path.is_file()
        && let Ok(html) = fs::read_to_string(custom_path)
    {
        return html;
    }
    render_default_start_page_html(bookmarks, history)
}

/// Render the built-in Tokyo Night-styled `:tabnew` starter page with a search bar,
/// recent bookmarks (newest first), and recent navigation history links.
pub fn render_default_start_page_html(bookmarks: &[Bookmark], history: &[HistoryEntry]) -> String {
    let bookmarks_html = render_bookmark_items(bookmarks);
    let history_html = render_history_items(history);

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>nservo — New Tab</title>
<style>
  :root {{ color-scheme: dark; }}
  body {{
    margin: 0;
    padding: 3rem 2rem 6rem;
    background: #1a1b26;
    color: #c0caf5;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    line-height: 1.6;
  }}
  main {{ max-width: 820px; margin: 0 auto; }}
  h1 {{ color: #7aa2f7; font-size: 1.8rem; margin: 0 0 0.25rem; }}
  .subtitle {{ color: #565f89; margin: 0 0 1.75rem; font-size: 0.9rem; }}
  .search-form {{ display: flex; gap: 0.6rem; margin-bottom: 2.5rem; }}
  .search-input {{
    flex: 1;
    background: #24283b;
    color: #c0caf5;
    border: 1px solid #3b4261;
    border-radius: 6px;
    padding: 0.7rem 0.95rem;
    font: inherit;
    font-size: 1rem;
    outline: none;
  }}
  .search-input:focus {{ border-color: #7aa2f7; }}
  .search-btn {{
    background: #7aa2f7;
    color: #16161e;
    border: none;
    border-radius: 6px;
    padding: 0.7rem 1.25rem;
    font: inherit;
    font-weight: bold;
    cursor: pointer;
  }}
  .search-btn:hover {{ background: #89b4fa; }}
  h2 {{
    color: #bb9af7;
    font-size: 1.05rem;
    margin: 2rem 0 0.75rem;
    border-bottom: 1px solid #292e42;
    padding-bottom: 0.3rem;
  }}
  ul {{ list-style: none; padding: 0; margin: 0; }}
  li {{ padding: 0.55rem 0.4rem; border-bottom: 1px solid #24283b; }}
  a {{ text-decoration: none; font-weight: bold; }}
  a:hover {{ text-decoration: underline; }}
  .bookmarks a {{ color: #9ece6a; }}
  .bookmarks a:hover {{ color: #7dcfff; }}
  .history a {{ color: #7dcfff; }}
  .history a:hover {{ color: #9ece6a; }}
  .url {{ color: #787c99; font-size: 0.82rem; word-break: break-all; }}
  .empty {{ color: #a9b1d6; padding: 0.6rem 0; font-size: 0.9rem; }}
  code {{ background: #24283b; padding: 0.1rem 0.35rem; border-radius: 3px; color: #e0af68; }}
</style>
</head>
<body>
<main>
  <h1>nservo</h1>
  <p class="subtitle">Press <code>f</code> or <code>F</code> to follow a link with hints, or <code>:open</code> to navigate.</p>
  <form class="search-form" action="{DEFAULT_SEARCH_ENGINE_ACTION}" method="GET">
    <input class="search-input" type="search" name="q" placeholder="Search with Google..." autofocus required>
    <button class="search-btn" type="submit">Search</button>
  </form>

  <section class="bookmarks">
    <h2>Bookmarks</h2>
    {bookmarks_html}
  </section>

  <section class="history">
    <h2>History</h2>
    {history_html}
  </section>
</main>
</body>
</html>"#
    )
}

fn render_bookmark_items(bookmarks: &[Bookmark]) -> String {
    if bookmarks.is_empty() {
        return r#"<p class="empty">No bookmarks saved yet. Press <code>b</code> on any page to bookmark it.</p>"#.to_string();
    }
    let mut out = String::from("<ul>");
    for bm in bookmarks {
        let title = escape_html(bm.display_title());
        let url = escape_html(&bm.url);
        let _ = write!(
            &mut out,
            r#"<li><a href="{url}">{title}</a><div class="url">{url}</div></li>"#
        );
    }
    out.push_str("</ul>");
    out
}

fn render_history_items(history: &[HistoryEntry]) -> String {
    if history.is_empty() {
        return r#"<p class="empty">No navigation history recorded yet.</p>"#.to_string();
    }
    let mut out = String::from("<ul>");
    for item in history {
        let title = escape_html(item.display_title());
        let url = escape_html(&item.url);
        let _ = write!(
            &mut out,
            r#"<li><a href="{url}">{title}</a><div class="url">{url}</div></li>"#
        );
    }
    out.push_str("</ul>");
    out
}

fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}
