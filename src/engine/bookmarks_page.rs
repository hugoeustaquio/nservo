use std::fmt::Write;

use crate::core::Bookmark;

/// URL assigned to the built-in bookmarks view.
pub const BOOKMARKS_PAGE_URL: &str = "about:bookmarks";

/// Render the built-in Tokyo Night-styled `about:bookmarks` HTML page.
pub fn render_bookmarks_html(bookmarks: &[Bookmark]) -> String {
    let mut rows = String::new();
    if bookmarks.is_empty() {
        rows.push_str(
            r#"<p class="empty">No bookmarks saved yet. Press <code>b</code> on any page to add it here.</p>"#,
        );
    } else {
        rows.push_str("<table><tbody>");
        for (idx, bm) in bookmarks.iter().enumerate() {
            let num = idx + 1;
            let title = escape_html(bm.display_title());
            let url = escape_html(&bm.url);
            let _ = write!(
                &mut rows,
                r#"<tr><td class="idx">{num}</td><td><a href="{url}">{title}</a><div class="url">{url}</div></td></tr>"#
            );
        }
        rows.push_str("</tbody></table>");
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>nservo — Bookmarks</title>
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
  h1 {{ color: #7aa2f7; font-size: 1.8rem; margin-bottom: 0.25rem; }}
  .subtitle {{ color: #565f89; margin-top: 0; margin-bottom: 2rem; }}
  table {{ width: 100%; border-collapse: collapse; }}
  td {{ padding: 0.65rem 0.6rem; border-bottom: 1px solid #24283b; vertical-align: top; }}
  td.idx {{ width: 42px; color: #565f89; text-align: right; padding-right: 1rem; }}
  a {{ color: #9ece6a; text-decoration: none; font-weight: bold; }}
  a:hover {{ text-decoration: underline; color: #7dcfff; }}
  .url {{ color: #787c99; font-size: 0.85rem; word-break: break-all; }}
  .empty {{ color: #a9b1d6; padding: 1.5rem 0; }}
  code {{ background: #24283b; padding: 0.1rem 0.35rem; border-radius: 3px; color: #e0af68; }}
</style>
</head>
<body>
<main>
  <h1>Bookmarks</h1>
  <p class="subtitle">Press <code>f</code> to follow a bookmark with hints, <code>b</code> on any page to add one, or <code>:bmdel &lt;index&gt;</code> to remove.</p>
  {rows}
</main>
</body>
</html>"#
    )
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
