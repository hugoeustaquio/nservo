# nservo-readability

A zero-idle-overhead **`page_js`** example plugin for [nservo](../../../README.md) that toggles a distraction-free, Readability-style reader view on the active tab.

## Features

- **Heuristic article extraction**: Scores `<article>`, `<main>`, `[role="main"]`, `<section>`, and `<div>` containers by paragraph length and link density, stripping scripts, navbars, sidebars, forms, ads, and inline styles.
- **Native scrolling**: Hides sibling `<body>` elements while active so `nservo`'s normal-mode scrolling (`j`, `k`, `gg`, `G`) and GTK4 overlay scrollbar scroll the reader article directly.
- **Instant toggle**: Running the plugin a second time (or clicking **Exit Reader**) restores the original page and previous scroll position.
- **Zero idle overhead**: Runs strictly on demand — no background processes, no persistent scripts injected into tabs you never activate it on.

## Installation

### From the `nservo` repository (local copy)

```bash
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/nservo/plugins"
cp -r examples/plugins/readability "${XDG_CONFIG_HOME:-$HOME/.config}/nservo/plugins/"
```

### From a standalone Git repository

If hosted in its own repository (e.g. `owner/nservo-readability`), install directly inside `nservo`:

```text
:plugin-install owner/nservo-readability
```

## Usage

- **Default shortcut**: `zr` (in Normal mode)
- **Command**: `:plugin readability` (or `:plug readability`)

Run the shortcut or command again to exit reader mode and return to the original page.

### Changing the Shortcut

Do **not** edit `plugin.json` inside the installed plugin directory — local changes there can conflict with or be lost on `:plugin-update`. Instead, override the shortcut in `$XDG_CONFIG_HOME/nservo/config.json` (or via `about:config` under **Plugin Keybindings**):

```json
{
  "plugin_keybindings": {
    "readability": "gr"
  }
}
```

Set the value to `""` to disable the default shortcut while keeping `:plugin readability` available.

## Files

| File | Purpose |
| --- | --- |
| `plugin.json` | Plugin manifest (`mode: "page_js"`, default shortcut `zr`) |
| `readability.js` | Article scoring, sanitization, and reader toggle script |
| `readability.css` | Tokyo Night-inspired reader theme injected alongside the script |

See [plugins.md](../../../plugins.md) for the full `nservo` plugin specification.
