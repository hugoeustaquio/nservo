# nservo Plugin Specification

`nservo` supports a lightweight, performance-first plugin system with **zero idle runtime overhead** and **no embedded scripting VM**. Plugins live in `$XDG_CONFIG_HOME/nservo/plugins/` (falling back to `$HOME/.config/nservo/plugins/`) and are installed and updated directly via `git`.

---

## Design Goals

- **Zero idle cost:** No background daemon, Lua/WASM runtime, or per-event IPC loop runs when plugins are not invoked.
- **Two fast execution modes:**
  - `page_js`: Executes JavaScript (with optional CSS) directly inside Servo's SpiderMonkey JIT with no subprocess overhead — ideal for DOM transforms like **Readability**.
  - `exec`: Spawns a native binary or CLI script on demand (on a background thread, bounded by a timeout) and dispatches JSON-line actions from `stdout` — ideal for host integrations like **password managers** (`rbw`, `pass`, `op`, `keepassxc-cli`, or compiled Rust/C/Go tools).
- **Minimal browser surface:** Reuses `nservo`'s existing Ex commands, clipboard, UTF-16 input injection, and `about:config` keybinding system.

---

## Directory Layout

Each plugin occupies its own directory containing a `plugin.json` manifest:

```text
$XDG_CONFIG_HOME/nservo/plugins/
├── readability/
│   ├── plugin.json
│   ├── readability.js
│   └── readability.css
└── pass-fill/
    ├── plugin.json
    └── pass-fill.py       # or a compiled native binary
```

---

## Managing & Running Plugins

| Command | Description |
| :--- | :--- |
| `:plugin-install <owner/repo \| git-url>` | Clones a GitHub shorthand (`owner/repo` -> `https://github.com/owner/repo.git`) or Git URL (`https://`, `ssh://`, `git@`, `file://`) with `git clone --depth=1` into `$XDG_CONFIG_HOME/nservo/plugins/<repo>`. |
| `:plugin-update [name]` | Runs `git pull --ff-only` on one plugin or all installed Git-backed plugins and reloads manifests. |
| `:plugin-remove <name>` | Removes an installed plugin directory and reloads manifests. |
| `:plugin <name> [args]` | Runs an installed plugin on the active tab (alias: `:plug`, supports fzf tab completion). |

---

## Manifest Reference (`plugin.json`)

| Field | Type | Required | Description |
| :--- | :--- | :--- | :--- |
| `name` | `string` | Yes | Plugin identifier (`[A-Za-z0-9_-]`, max 64 chars). Used by `:plugin <name>` and `config.json`. |
| `description` | `string` | No | Human-readable summary shown in completion and `about:config`. |
| `default_shortcut` | `string` | No | Optional 1- or 2-character Normal-mode shortcut (e.g. `"zr"`, `"zp"`). Applied automatically if it does not conflict with existing bindings. |
| `mode` | `"page_js" \| "exec"` | Yes | Execution strategy. |
| `script` | `string` | For `page_js` | Relative path (inside the plugin dir) to the JavaScript file to evaluate. |
| `style` | `string` | No | Optional relative path to a CSS file injected before `script` runs (`page_js` mode). |
| `command` | `string` | For `exec` | Executable path relative to the plugin dir (e.g. `"./pass-fill"`) or binary name in `PATH`. |
| `timeout_ms` | `number` | No | Max execution time for `exec` plugins in milliseconds (default `10000`, clamped to `200..=60000`). |

> Paths in `script`, `style`, and relative `command` must stay inside the plugin directory (`..` traversal is rejected).

---

## Mode 1: `page_js` Plugins

`page_js` plugins run directly inside the active tab's SpiderMonkey JS context with zero subprocess spawn. Before executing `script`:

1. If `style` is declared, `nservo` inserts or updates `<style id="nservo-plugin-style-<name>">`.
2. `nservo` sets `window.__nservoPlugin` to:
   ```json
   { "name": "...", "url": "...", "origin": "...", "title": "...", "args": "..." }
   ```
3. `script` runs inside an IIFE `(function(){ ... })();`.

Example manifest ([examples/plugins/readability/plugin.json](examples/plugins/readability/plugin.json)):

```json
{
  "name": "readability",
  "description": "Toggle distraction-free reader view (Readability-style)",
  "default_shortcut": "zr",
  "mode": "page_js",
  "script": "readability.js",
  "style": "readability.css"
}
```

---

## Mode 2: `exec` Plugins

`exec` plugins are spawned asynchronously on demand with working directory set to the plugin folder.

### Inputs Provided to the Plugin

- **Environment variables:**
  - `NSERVO_URL` — Active tab URL
  - `NSERVO_ORIGIN` — Active tab `scheme://host[:port]`
  - `NSERVO_TITLE` — Active tab page title
  - `NSERVO_ARGS` — Optional arguments passed to `:plugin <name> <args>`
  - `NSERVO_PLUGIN_DIR` — Absolute path to the plugin directory
  - `NSERVO_CONFIG_DIR` — Absolute path to `$XDG_CONFIG_HOME/nservo`
- **`stdin`:** One JSON line containing `{"url":"...","origin":"...","title":"...","args":"..."}`.

### Output Protocol (`stdout` JSON Lines)

Write one JSON object per line to `stdout`. Blank lines are ignored. Non-zero exit codes display `stderr` in the status bar.

| `op` | Fields | Effect in `nservo` |
| :--- | :--- | :--- |
| `eval_js` | `code: string` | Evaluates JavaScript in the active tab. |
| `insert_text` | `text: string` | Inserts UTF-16 text into the currently focused page input. |
| `copy` | `text: string` | Writes text to the system clipboard. |
| `open` | `url: string`, `new_tab?: bool` | Navigates current tab (or opens a new tab when `new_tab: true`). |
| `command` | `cmd: string` | Runs any `nservo` Ex command (e.g. `"reload"`, `"tabnew"`). |
| `notice` | `text: string` | Shows an informational message in the status bar. |
| `error` | `text: string` | Shows an error message in the status bar. |

Example manifest ([examples/plugins/pass-fill/plugin.json](examples/plugins/pass-fill/plugin.json)):

```json
{
  "name": "pass-fill",
  "description": "Autofill credentials from rbw (Bitwarden), pass, or 1Password CLI",
  "default_shortcut": "zp",
  "mode": "exec",
  "command": "./pass-fill.py",
  "timeout_ms": 10000
}
```

---

## Keyboard Shortcuts & `about:config`

Plugin shortcuts live alongside built-in shortcuts in `$XDG_CONFIG_HOME/nservo/config.json`:

```json
{
  "keybindings": { "...": "..." },
  "plugin_keybindings": {
    "readability": "zr",
    "pass-fill": "zp"
  }
}
```

- Every installed plugin appears automatically under the **Plugins** section in **`about:config`** (`:config`).
- Saving in `about:config` validates all shortcuts (1–2 characters, no duplicates, no single-key/prefix collisions, cannot start with count digits `1`–`9`), persists `config.json`, and hot-reloads keybindings immediately.

---

## Bundled Example Plugins

Two ready-to-use example plugins are included in `examples/plugins/`:

1. **[examples/plugins/readability](examples/plugins/readability)** (`zr`) — `page_js` distraction-free reader overlay that extracts and reformats the main article body and toggles cleanly on/off.
2. **[examples/plugins/pass-fill](examples/plugins/pass-fill)** (`zp`) — `exec` password manager bridge supporting `rbw` (Bitwarden), `pass`, and `op` (1Password CLI) with React/Vue-compatible form autofill.

Try them locally:

```bash
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/nservo/plugins"
cp -r examples/plugins/readability examples/plugins/pass-fill \
  "${XDG_CONFIG_HOME:-$HOME/.config}/nservo/plugins/"
```

Then restart `nservo` (or open `:config` to customize their shortcuts) and press `zr` on an article or `zp` on a login page.

---

## Security Notes

- `exec` plugins run with your OS user privileges — install only plugins you trust and review `plugin.json` and scripts before running them.
- `exec` plugins **never** run automatically on page load; they execute only on an explicit shortcut or `:plugin` command.
- Every `exec` invocation is killed automatically if it exceeds `timeout_ms`.
