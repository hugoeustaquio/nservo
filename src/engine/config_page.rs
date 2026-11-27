use std::fmt::Write;
use std::path::Path;

use crate::core::{
    BrowserConfig, InstalledPlugin, SHORTCUT_SPECS, default_config_path, default_plugins_dir,
    discover_plugins,
};

/// Built-in URL for the interactive configuration & keyboard-shortcuts editor.
pub const CONFIG_PAGE_URL: &str = "about:config";

/// `UserContentManager` script message handler used by `about:config`.
pub const CONFIG_MESSAGE_HANDLER: &str = "nservoConfig";

/// Render `about:config` for the currently active `config`, installed plugins, and default config path.
pub fn render_default_config_html(config: &BrowserConfig) -> String {
    let plugins = discover_plugins(&default_plugins_dir());
    render_config_html(config, &default_config_path(), &plugins)
}

/// Render the Tokyo Night-styled interactive `about:config` HTML page.
pub fn render_config_html(
    config: &BrowserConfig,
    config_path: &Path,
    plugins: &[InstalledPlugin],
) -> String {
    let path_html = escape_html(&config_path.display().to_string());
    let mut rows = String::new();
    let mut last_cat = "";

    for spec in SHORTCUT_SPECS {
        if spec.category != last_cat {
            let cat = escape_html(spec.category);
            let _ = write!(
                &mut rows,
                r#"<tr class="cat-row"><td colspan="4">{cat}</td></tr>"#
            );
            last_cat = spec.category;
        }
        let id = escape_html(spec.id);
        let desc = escape_html(spec.description);
        let def = escape_html(spec.default_key);
        let cur = escape_html(config.key_for(spec.id));
        let modified_cls = if config.key_for(spec.id) != spec.default_key {
            " modified"
        } else {
            ""
        };
        let search_key = escape_html(&format!("{} {} {} {}", spec.id, spec.category, spec.description, cur).to_ascii_lowercase());
        let _ = write!(
            &mut rows,
            r#"<tr class="cfg-row{modified_cls}" data-search="{search_key}">
  <td class="id-col"><code>{id}</code></td>
  <td class="desc-col">{desc}</td>
  <td class="def-col"><code>{def}</code></td>
  <td class="val-col"><input class="key-input" type="text" maxlength="2" data-id="{id}" data-default="{def}" value="{cur}"></td>
</tr>"#
        );
    }

    if !plugins.is_empty() {
        rows.push_str(r#"<tr class="cat-row"><td colspan="4">Plugins</td></tr>"#);
        for p in plugins {
            let id = escape_html(&p.manifest.name);
            let desc = escape_html(&p.manifest.description);
            let def_raw = p.manifest.default_shortcut.as_deref().unwrap_or("");
            let def = escape_html(def_raw);
            let cur_raw = config.plugin_key_for(&p.manifest.name);
            let cur = escape_html(cur_raw);
            let modified_cls = if cur_raw != def_raw { " modified" } else { "" };
            let search_key = escape_html(&format!("plugin {} {} {}", p.manifest.name, p.manifest.description, cur_raw).to_ascii_lowercase());
            let _ = write!(
                &mut rows,
                r#"<tr class="cfg-row{modified_cls}" data-search="{search_key}">
  <td class="id-col"><code>plugin:{id}</code></td>
  <td class="desc-col">{desc}</td>
  <td class="def-col"><code>{def}</code></td>
  <td class="val-col"><input class="key-input plugin-key-input" type="text" maxlength="2" data-plugin="{id}" data-default="{def}" value="{cur}"></td>
</tr>"#
            );
        }
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>nservo — Config</title>
<style>
  :root {{ color-scheme: dark; }}
  body {{
    margin: 0; padding: 2.5rem 2rem 5rem; background: #1a1b26; color: #c0caf5;
    font-family: "JetBrains Mono", "Fira Code", monospace; line-height: 1.55;
  }}
  main {{ max-width: 900px; margin: 0 auto; }}
  h1 {{ color: #7aa2f7; font-size: 1.75rem; margin: 0 0 0.25rem; }}
  .subtitle {{ color: #787c99; margin: 0 0 1.25rem; font-size: 0.88rem; }}
  .toolbar {{ display: flex; gap: 0.6rem; align-items: center; margin-bottom: 1rem; flex-wrap: wrap; }}
  .filter {{
    flex: 1; min-width: 220px; background: #24283b; color: #c0caf5;
    border: 1px solid #3b4261; border-radius: 5px; padding: 0.45rem 0.75rem; font: inherit;
  }}
  .btn {{
    background: #7aa2f7; color: #16161e; border: none; border-radius: 5px;
    padding: 0.48rem 1rem; font: inherit; font-weight: bold; cursor: pointer;
  }}
  .btn:hover {{ background: #89b4fa; }}
  .btn.secondary {{ background: #292e42; color: #c0caf5; }}
  .btn.secondary:hover {{ background: #3b4261; }}
  #banner {{
    display: none; padding: 0.55rem 0.85rem; border-radius: 5px; margin-bottom: 1rem;
    font-size: 0.9rem; font-weight: bold;
  }}
  #banner.ok {{ display: block; background: #22372c; color: #9ece6a; border: 1px solid #9ece6a; }}
  #banner.err {{ display: block; background: #3b2029; color: #f7768e; border: 1px solid #f7768e; }}
  table {{ width: 100%; border-collapse: collapse; }}
  th {{ text-align: left; color: #787c99; font-size: 0.8rem; border-bottom: 1px solid #292e42; padding: 0.4rem 0.5rem; }}
  td {{ padding: 0.45rem 0.5rem; border-bottom: 1px solid #24283b; vertical-align: middle; }}
  tr.cat-row td {{ color: #bb9af7; font-weight: bold; padding-top: 1.15rem; border-bottom: 1px solid #292e42; }}
  tr.modified .key-input {{ border-color: #e0af68; color: #e0af68; }}
  .key-input {{
    width: 64px; text-align: center; background: #16161e; color: #9ece6a;
    border: 1px solid #3b4261; border-radius: 4px; padding: 0.28rem 0.4rem;
    font: inherit; font-weight: bold;
  }}
  .key-input:focus {{ border-color: #7aa2f7; outline: none; }}
  code {{ background: #24283b; padding: 0.1rem 0.35rem; border-radius: 3px; color: #e0af68; }}
  .id-col {{ width: 210px; }}
  .def-col {{ width: 90px; text-align: center; }}
  .val-col {{ width: 100px; text-align: center; }}
</style>
</head>
<body>
<main>
  <h1>nservo — about:config</h1>
  <p class="subtitle">Saved to <code>{path_html}</code>. Press <code>i</code> (or click an input) to edit 1–2 char Normal-mode shortcuts; changes apply immediately on Save.</p>
  <div id="banner"></div>
  <div class="toolbar">
    <input id="filter" class="filter" type="search" placeholder="Filter shortcuts by name, key, or description...">
    <button id="save-btn" class="btn" type="button">Save</button>
    <button id="reset-btn" class="btn secondary" type="button">Reset Defaults</button>
  </div>
  <table>
    <thead><tr><th>Action</th><th>Description</th><th>Default</th><th>Shortcut</th></tr></thead>
    <tbody>{rows}</tbody>
  </table>
</main>
<script>
(function() {{
  var banner = document.getElementById('banner');
  window.__nservoConfigStatus = function(ok, msg) {{
    banner.className = ok ? 'ok' : 'err';
    banner.textContent = msg;
  }};
  function postConfig(payload) {{
    var h = window.servoGtk && window.servoGtk.messageHandlers && window.servoGtk.messageHandlers.{CONFIG_MESSAGE_HANDLER};
    if (h) h.postMessage(payload);
    else window.__nservoConfigStatus(false, 'Config bridge unavailable');
  }}
  document.getElementById('save-btn').addEventListener('click', function() {{
    var map = {{}};
    var pmap = {{}};
    var inputs = document.querySelectorAll('.key-input');
    for (var i = 0; i < inputs.length; i++) {{
      var val = inputs[i].value.trim();
      if (inputs[i].dataset.plugin) {{
        if (val) pmap[inputs[i].dataset.plugin] = val;
      }} else {{
        map[inputs[i].dataset.id] = val;
      }}
    }}
    postConfig({{ action: 'save', keybindings: map, plugin_keybindings: pmap }});
  }});
  document.getElementById('reset-btn').addEventListener('click', function() {{
    postConfig({{ action: 'reset' }});
  }});
  document.getElementById('filter').addEventListener('input', function(ev) {{
    var q = (ev.target.value || '').trim().toLowerCase();
    var rows = document.querySelectorAll('tr.cfg-row');
    for (var i = 0; i < rows.length; i++) {{
      var hay = rows[i].dataset.search || '';
      rows[i].style.display = (!q || hay.indexOf(q) !== -1) ? '' : 'none';
    }}
  }});
  var inputs = document.querySelectorAll('.key-input');
  for (var j = 0; j < inputs.length; j++) {{
    inputs[j].addEventListener('input', function(ev) {{
      var inp = ev.target;
      var row = inp.closest('tr');
      if (row) row.classList.toggle('modified', inp.value.trim() !== inp.dataset.default);
    }});
  }}
}})();
</script>
</body>
</html>"#
    )
}

/// Build JS to update the inline feedback banner on `about:config`.
pub fn config_feedback_script(ok: bool, message: &str) -> String {
    let ok_js = if ok { "true" } else { "false" };
    let msg_json = serde_json::to_string(message).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        "if (window.__nservoConfigStatus) {{ window.__nservoConfigStatus({ok_js}, {msg_json}); }}"
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
