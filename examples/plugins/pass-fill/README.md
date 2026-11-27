# nservo-pass-fill

A zero-idle-overhead **`exec`** example plugin for [nservo](../../../README.md) that queries your local CLI password manager on demand and autofills the username and password fields on the active tab.

## Supported Backends

Backends are checked automatically in the following order:

1. **[`rbw`](https://github.com/doy/rbw)** (Bitwarden CLI) — `rbw get --full <host>`
2. **[`pass`](https://www.passwordstore.org/)** (Standard Unix Password Manager) — `pass show <host>`
   - Reads the password from line 1 and username from a `username:`, `user:`, `login:`, or `email:` line.
3. **[`op`](https://developer.1password.com/docs/cli/)** (1Password CLI) — `op item get <host> --fields username,password --format json`

## Requirements

- `python3` in `PATH`
- At least one configured and unlocked CLI password manager (`rbw`, `pass`, or `op`)
- `pass-fill.py` marked executable (`chmod +x pass-fill.py`)

## Installation

### From the `nservo` repository (local copy)

```bash
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/nservo/plugins"
cp -r examples/plugins/pass-fill "${XDG_CONFIG_HOME:-$HOME/.config}/nservo/plugins/"
chmod +x "${XDG_CONFIG_HOME:-$HOME/.config}/nservo/plugins/pass-fill/pass-fill.py"
```

### From a standalone Git repository

If hosted in its own repository (e.g. `owner/nservo-pass-fill`), install directly inside `nservo`:

```text
:plugin-install owner/nservo-pass-fill
```

## Usage

- **Default shortcut**: `zp` (in Normal mode) — looks up credentials using the current tab's hostname (stripping a leading `www.`).
- **Command with automatic hostname**: `:plugin pass-fill` (or `:plug pass-fill`)
- **Command with custom entry name**: `:plugin pass-fill github.com` (overrides the hostname lookup when your vault entry has a different name)

When a matching entry is found, the script injects values into the first enabled password and username/email `<input>` elements using the native property setter and dispatches `input` + `change` events (compatible with React/Vue/SPA login forms).

### Changing the Shortcut

Do **not** edit `plugin.json` inside the installed plugin directory — local changes there can conflict with or be lost on `:plugin-update`. Instead, override the shortcut in `$XDG_CONFIG_HOME/nservo/config.json` (or via `about:config` under **Plugin Keybindings**):

```json
{
  "plugin_keybindings": {
    "pass-fill": "gp"
  }
}
```

Set the value to `""` to disable the default shortcut while keeping `:plugin pass-fill` available.

## Security Notes

- The plugin process is only spawned when you explicitly trigger it and exits immediately afterward (capped by `timeout_ms: 10000`).
- Credentials are never written to disk by `nservo`; they are passed directly as an `eval_js` action over stdout to the active tab.

## Files

| File | Purpose |
| --- | --- |
| `plugin.json` | Plugin manifest (`mode: "exec"`, default shortcut `zp`, 10s timeout) |
| `pass-fill.py` | On-demand Python script that queries `rbw`/`pass`/`op` and emits JSON-lines actions |

See [plugins.md](../../../plugins.md) for the full `nservo` plugin specification.
