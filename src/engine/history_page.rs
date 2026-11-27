use std::fmt::Write;

use crate::core::HistoryEntry;

/// URL assigned to the built-in browsing history view.
pub const HISTORY_PAGE_URL: &str = "about:history";

/// Render the built-in Tokyo Night-styled `about:history` HTML page.
pub fn render_history_html(entries: &[HistoryEntry], filter: Option<&str>) -> String {
    let subtitle = match filter.map(str::trim).filter(|q| !q.is_empty()) {
        Some(q) => format!(
            "Filtered by <code>{}</code> — press <code>f</code> to follow a link or <code>:history</code> to show all.",
            escape_html(q)
        ),
        None => "Press <code>f</code> to follow a link, <code>:history &lt;query&gt;</code> to filter, or <code>:history-clear</code> to clear.".to_string(),
    };

    let mut rows = String::new();
    if entries.is_empty() {
        rows.push_str(r#"<p class="empty">No matching browsing history entries.</p>"#);
    } else {
        rows.push_str("<table><tbody>");
        for (idx, item) in entries.iter().enumerate() {
            let num = idx + 1;
            let title = escape_html(item.display_title());
            let url = escape_html(&item.url);
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
<title>nservo — History</title>
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
  a {{ color: #7dcfff; text-decoration: none; font-weight: bold; }}
  a:hover {{ text-decoration: underline; color: #9ece6a; }}
  .url {{ color: #787c99; font-size: 0.85rem; word-break: break-all; }}
  .empty {{ color: #a9b1d6; padding: 1.5rem 0; }}
  code {{ background: #24283b; padding: 0.1rem 0.35rem; border-radius: 3px; color: #e0af68; }}
</style>
</head>
<body>
<main>
  <h1>History</h1>
  <p class="subtitle">{subtitle}</p>
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
