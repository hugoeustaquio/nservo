/// Neovim-inspired start and `:help` page rendered directly inside Servo.
pub const HELP_PAGE_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>nservo — Help</title>
<style>
  :root { color-scheme: dark; }
  body {
    margin: 0;
    padding: 3rem 2rem 6rem;
    background: #1a1b26;
    color: #c0caf5;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    line-height: 1.6;
  }
  main { max-width: 760px; margin: 0 auto; }
  h1 { color: #7aa2f7; font-size: 1.8rem; margin-bottom: 0.25rem; }
  .subtitle { color: #565f89; margin-top: 0; margin-bottom: 2rem; }
  h2 { color: #bb9af7; font-size: 1.1rem; margin-top: 2rem; border-bottom: 1px solid #292e42; padding-bottom: 0.3rem; }
  table { width: 100%; border-collapse: collapse; margin-top: 0.75rem; }
  td { padding: 0.45rem 0.6rem; border-bottom: 1px solid #24283b; vertical-align: top; }
  td:first-child { width: 220px; color: #9ece6a; font-weight: bold; }
  code { background: #24283b; padding: 0.1rem 0.35rem; border-radius: 3px; color: #e0af68; }
  .scroll-demo { margin-top: 3rem; color: #565f89; }
</style>
</head>
<body>
<main>
  <h1>nservo</h1>
  <p class="subtitle">Neovim-inspired GTK4 browser powered by servo-gtk</p>

  <h2>Modes</h2>
  <table>
    <tr><td>NORMAL</td><td>Default navigation &amp; shortcut mode (press <code>Esc</code> from any mode).</td></tr>
    <tr><td>INPUT</td><td>Press <code>i</code> to pass keystrokes directly to the page.</td></tr>
    <tr><td>COMMAND</td><td>Press <code>:</code> to enter Ex-style commands at the bottom bar.</td></tr>
    <tr><td>HINT</td><td>Press <code>f</code> to label clickable elements and type their letters to click/focus them.</td></tr>
  </table>

  <h2>Normal Mode Shortcuts</h2>
  <table>
    <tr><td>j / k</td><td>Scroll down / scroll up</td></tr>
    <tr><td>h / l</td><td>Scroll left / scroll right</td></tr>
    <tr><td>gg / G</td><td>Scroll to top / scroll to bottom</td></tr>
    <tr><td>gt / gT</td><td>Next tab / previous tab (400ms sequence window)</td></tr>
    <tr><td>f</td><td>Show letter hints on links, buttons, and inputs</td></tr>
    <tr><td>d</td><td>Close current tab</td></tr>
    <tr><td>o / O</td><td>Open Command mode pre-filled with <code>:open </code> / <code>:Open </code></td></tr>
    <tr><td>H / L</td><td>Go back / go forward in history</td></tr>
    <tr><td>r</td><td>Reload current tab</td></tr>
    <tr><td>i</td><td>Enter INPUT mode</td></tr>
    <tr><td>:</td><td>Enter COMMAND mode</td></tr>
  </table>

  <h2>Commands</h2>
  <table>
    <tr><td>:open &lt;URL&gt;</td><td>Open URL in the current tab (alias: <code>:o</code>)</td></tr>
    <tr><td>:Open &lt;URL&gt;</td><td>Open URL in a new tab (aliases: <code>:O</code>, <code>:tabopen</code>)</td></tr>
    <tr><td>:tabnext / :tabprev</td><td>Switch to next / previous tab</td></tr>
    <tr><td>:tabclose</td><td>Close current tab (aliases: <code>:tc</code>, <code>:bd</code>)</td></tr>
    <tr><td>:reload / :back / :forward</td><td>Page navigation controls</td></tr>
    <tr><td>:zoom &lt;factor|reset&gt;</td><td>Set page zoom level (e.g. <code>:zoom 1.25</code>)</td></tr>
    <tr><td>:w [name]</td><td>Save current tabs as session (defaults to <code>default</code>)</td></tr>
    <tr><td>:wq [name]</td><td>Save session and quit</td></tr>
    <tr><td>:session &lt;name&gt;</td><td>Restore a saved session by name</td></tr>
    <tr><td>:help</td><td>Show this reference page</td></tr>
    <tr><td>:q / :quit</td><td>Quit nservo</td></tr>
  </table>

  <div class="scroll-demo">
    <p>Tip: Try pressing <code>j</code> and <code>k</code> to test scrolling, or <code>:Open example.com</code> to open a new tab and switch with <code>gt</code> / <code>gT</code>.</p>
  </div>
</main>
</body>
</html>"#;
