# nservo

[![License: BSD-3-Clause](https://img.shields.io/badge/License-BSD%203--Clause-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024%20edition-orange.svg)](https://www.rust-lang.org/)
[![Engine: Servo](https://img.shields.io/badge/Engine-Servo%20(servo--gtk)-teal.svg)](https://github.com/nacho/servo-gtk)
[![UI: GTK4](https://img.shields.io/badge/UI-GTK4-4a86cf.svg)](https://gtk.org/)

A fast, keyboard-driven web browser written in Rust with Vim/Neovim modal interaction, heavily inspired by [qutebrowser](https://qutebrowser.org/) and powered by [Servo](https://servo.org/) via [`servo-gtk`](https://github.com/nacho/servo-gtk) and GTK4.

---

## Features

- **Modal Vim-style browsing:** Distinct `NORMAL`, `INSERT`, `COMMAND`, `SEARCH`, and `HINT` modes with GTK4 Capture-phase key interception so shortcuts never leak into pages.
- **In-page Vim search (`/`):** Smartcase search with match highlighting, automatic scroll-to-match, wrapping `n` / `N` navigation, and `Esc` / `:noh` highlight clearing.
- **Link hinting (`f`):** Prefix-free alphabetic overlays (`a`–`z`, `aa`–`zz`) to click links, buttons, and focus inputs entirely from the keyboard. Focusing an input automatically switches to `INSERT` mode.
- **Smooth scrolling & clean chrome:** `requestAnimationFrame`-eased scrolling (`j`/`k`/`h`/`l`, `gg`/`G`, and mouse wheel) paired with auto-hiding, hover-expanding GTK4 overlay scrollbars and a dark Tokyo Night-inspired interface.
- **Tabs, bookmarks, history & sessions (SQLite):** Numbered tabline, wrap-around switching (`gt`/`gT`), undo closed tabs (`u`), quick bookmarking (`b` / `B`), automatic browsing history (`gh` / `:history`), and named sessions (`:w`, `:wq`, `:session`) with automatic `default` restore—all persisted in a single SQLite database at `$XDG_CONFIG_HOME/nservo/nservo.db`.
- **Smart command bar (`:`):** Ex-style commands with automatic URL normalization (explicit schemes, `localhost`, IPv4/IPv6, bare domains) and Google Search fallback for plain text queries.
- **Built-in Web Inspector (`wi` / `:inspect`):** Docked GTK4 developer drawer with a per-tab JavaScript Console/REPL (with `Up`/`Down` history) and a live DOM tree + computed-style inspector with element highlighting.
- **Native HTML5 media:** Enables Servo's GStreamer backend (`media-gstreamer`) for `<video>`, `<audio>`, MSE, and WebAudio playback.
- **GPU-first rendering with CPU fallback:** Automatically detects Linux DRM render nodes (`/dev/dri/renderD*`), prefers hardware Vulkan/GL via GTK4, falls back cleanly to Cairo on CPU-only environments, and displays the active backend in the statusline.
- **Full Unicode, IME & clipboard support:** Bridges GTK4's `IMMulticontext` and a built-in dead-key composer so diacritics, accents, Compose sequences, and supplementary Unicode characters work cleanly in `INSERT` mode alongside `Ctrl+C`/`X`/`V`, `Ctrl+A`/`E`, and `Ctrl[+Shift]+Arrow`.

---

## Requirements

- **Linux** (Wayland or X11)
- **Rust** toolchain supporting the **2024 edition** (`rustc` 1.85+)
- **GTK 4** (`>= 4.12`) development headers (`gtk4`)
- **libepoxy** (`libepoxy.so.0`)
- **GStreamer** development libraries and plugins (for Servo's `media-gstreamer` HTML5 audio/video backend)
- Standard **Servo** build dependencies (`pkg-config`, `clang`/LLVM, `cmake`, `python3`, etc.)

---

## Building & Running

```bash
git clone https://github.com/hugoeustaquio/nservo.git
cd nservo

# Build in release mode (recommended for browsing performance)
cargo build --release

# Launch (restores the 'default' session if saved, otherwise opens the built-in help page)
cargo run --release

# Launch directly to a URL or search query
cargo run --release -- https://servo.org
```

> **Note:** The first build compiles `servo` and `servo-gtk` from Git and may take a while. Development builds automatically apply `opt-level = 2` to Servo's rendering crates (`Cargo.toml`) and increase Stylo's thread stack size via `.cargo/config.toml`.
>
> You can override the GTK4 render backend explicitly with `GSK_RENDERER` (for example `GSK_RENDERER=vulkan`, `GSK_RENDERER=gl`, or `GSK_RENDERER=cairo`).

---

## Modes

| Mode | How to Enter | Behavior |
| :--- | :--- | :--- |
| **NORMAL** | Default (`Esc` from any mode) | Keyboard navigation, scrolling, tab management, and multi-key sequences (`400ms` window). |
| **INSERT** | `i` (or activating a text input via `f`) | Sends keystrokes directly to the web page with IME/dead-key and clipboard support. Returns to `NORMAL` on `Esc` or page navigation. |
| **COMMAND** | `:`, `o`, `O`, or `go` | Shows the bottom `:` Ex-style command prompt. `Esc` or `BackSpace` on an empty prompt returns to `NORMAL`. |
| **SEARCH** | `/` | Shows the bottom `/` in-page search prompt (smartcase). Press `Enter` to highlight matches and jump to the first hit (empty `Enter` repeats the last search); `Esc` or empty `BackSpace` cancels. |
| **HINT** | `f` | Overlays clickable elements with letter badges. Type letters to filter/activate, `BackSpace` to undo, `Esc` to cancel. |

---

## Keybindings

### Normal Mode

| Key | Action |
| :--- | :--- |
| `[count]j` / `[count]k` | Smooth scroll down / up (optional numeric count, e.g. `12j`) |
| `[count]h` / `[count]l` | Smooth scroll left / right (optional numeric count, e.g. `5l`) |
| `gg` / `G` | Smooth scroll to top / bottom |
| `f` / `F` | Enter **HINT** mode (`f` follow links/inputs in current tab, `F` open link in new tab) |
| `/` | Enter **SEARCH** mode (in-page smartcase search) |
| `n` / `N` | Jump to next / previous search match (wraps around) |
| `Esc` | Clear active search highlights |
| `b` / `B` | Add current page to bookmarks / open `about:bookmarks` |
| `o` / `O` | Open command bar pre-filled with `:open ` (current tab) / `:Open ` (new tab) |
| `go` | Edit current tab's URL in command bar (`:open <current URL>`) |
| `gh` | Open browsing history (`about:history`) |
| `gt` / `gT` | Next tab / previous tab |
| `d` | Close current tab (closes window if last tab) |
| `u` | Reopen most recently closed tab at its previous position |
| `H` / `L` | History back / forward |
| `r` / `F5` | Reload current tab |
| `wi` | Toggle built-in Web Inspector (JS Console/REPL & DOM Inspector) |
| `i` | Enter **INSERT** mode |
| `:` | Enter **COMMAND** mode |
| `Ctrl+C` | Copy selected text to the system clipboard |

### Global & Insert Mode Chords

| Key | Scope | Action |
| :--- | :--- | :--- |
| `Ctrl++` / `Ctrl+=` | Global | Zoom in (`+10%`) |
| `Ctrl+-` | Global | Zoom out (`-10%`) |
| `Ctrl+0` | Global | Reset zoom to `100%` |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Insert (`Ctrl+C` also in Normal) | Copy / cut / paste via system clipboard |
| `Ctrl+Left` / `Ctrl+Right` | Insert | Move cursor by one word |
| `Ctrl+Shift+Left` / `Ctrl+Shift+Right` | Insert | Extend selection by one word |
| `Ctrl+A` / `Ctrl+E` | Insert | Move cursor to start / end of line |
| `Esc` | Any mode / Inspector | Return to **NORMAL** mode and focus the page |

---

## Commands

Press `:` to open the command bar and run Ex-style commands:

| Command | Aliases | Description |
| :--- | :--- | :--- |
| `:open <URL\|query>` | `:o` | Open a URL or Google Search in the current tab |
| `:Open <URL\|query>` | `:O`, `:tabopen` | Open a URL or Google Search in a new tab |
| `:tabnew [URL\|query]` | — | Open a new tab with the starter page (`$XDG_CONFIG_HOME/nservo/startpage.html` if present, else search bar + recent bookmarks + history) or an optional URL |
| `:tabnext` | `:tabn` | Switch to the next tab |
| `:tabprev` | `:tabp`, `:tabprevious` | Switch to the previous tab |
| `:tabclose` | `:tabc`, `:tc`, `:bd`, `:close` | Close the current tab |
| `:reload` | `:r` | Reload the current tab |
| `:back` | — | Navigate back in history |
| `:forward` | — | Navigate forward in history |
| `:zoom <factor\|reset>` | — | Set zoom level (`0.1`–`10.0`, e.g. `:zoom 1.25`) or `reset` |
| `:inspect` | `:inspector`, `:devtools` | Toggle the built-in Web Inspector |
| `:noh` | `:nohlsearch` | Clear active in-page search highlights |
| `:w [name]` | `:write [name]` | Save open tabs as a session (defaults to `default`) |
| `:wq [name]` | — | Save session and quit |
| `:session <name>` | — | Restore a saved session |
| `:bookmark` | `:bm-add` | Add the current page to bookmarks (shortcut: `b`) |
| `:bookmarks` | `:bm` | Open saved bookmarks (`about:bookmarks`, shortcut: `B`) |
| `:bmdel [index\|URL]` | `:bookmark-del` | Remove a bookmark by 1-based index, URL, or the current page |
| `:history [query]` | `:hist` | Show recent browsing history or filter by substring (shortcut: `gh`) |
| `:history-clear` | `:histclear` | Clear all saved browsing history |
| `:adblock <URL>` | — | Download an ABP-like filter list into `$XDG_CONFIG_HOME/nservo/adblock/` |
| `:adblock-update` | — | Re-download all saved adblock filter lists and reload rules |
| `:config` | — | Open interactive settings & keyboard shortcuts page (`about:config`) |
| `:help` | `:h` | Open the built-in keyboard & command reference page |
| `:q` | `:quit`, `:qa` | Quit `nservo` |

---

## Customization & Theming

### Custom CSS Theme (`style.css`)

`nservo`'s default Tokyo Night-inspired GTK4 stylesheet lives in [`assets/style.css`](assets/style.css). On startup, `nservo` checks for a user stylesheet at:

```text
$XDG_CONFIG_HOME/nservo/style.css   (fallback: $HOME/.config/nservo/style.css)
```

If that file exists, `nservo` loads it instead of the built-in stylesheet so you can customize colors, fonts, padding, borders, and widget layout using standard GTK4 CSS.

To start customizing from the default theme:

```bash
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/nservo"
cp assets/style.css "${XDG_CONFIG_HOME:-$HOME/.config}/nservo/style.css"
```

Key CSS selectors used by `nservo`'s chrome:

| Selector | UI Element |
| :--- | :--- |
| `window.nservo-window` | Top-level window background, foreground, and default font |
| `.tab-bar`, `.tab-button`, `button.tab-button.tab-active` | Tab bar container, inactive tabs, and active tab styling |
| `.status-bar`, `.status-url`, `.status-pending`, `.status-meta` | Bottom statusline, current URL/notice text (`.error` for errors), pending key chords, and tab/zoom/backend pill |
| `.mode-badge` (`.mode-normal`, `.mode-insert`, `.mode-command`, `.mode-search`, `.mode-hint`) | Mode indicator pill in the statusline |
| `.command-bar`, `.command-prefix`, `.command-entry` | Bottom `:` / `/` prompt bar and text entry |
| `.completion-scroll`, `.completion-list`, `.completion-badge`, `.completion-primary`, `.completion-secondary` | Command-bar fuzzy completion popup and category badges (`.kind-bm`, `.kind-hist`, `.kind-sess`) |
| `.inspector-panel`, `.inspector-header`, `.inspector-btn`, `.inspector-text`, `.inspector-repl`, `.inspector-dom-list` | Built-in Web Inspector drawer, console output, REPL, and DOM tree |
| `scrollbar.page-scrollbar` (`.visible`, `.hovering`, `.vertical`, `.horizontal`) | Auto-hiding GTK4 overlay scrollbars |

Restart `nservo` after editing `style.css` to apply your changes.

### Custom Keyboard Shortcuts (`config.json` & `about:config`)

Normal-mode keyboard shortcuts can be customized either by editing `$XDG_CONFIG_HOME/nservo/config.json` (fallback: `$HOME/.config/nservo/config.json`) directly or interactively inside the browser at **`about:config`** (via `:config` or `:open about:config`). Saving in `about:config` validates bindings (1–2 characters, no duplicates or prefix collisions), persists them to `config.json`, and applies them immediately without restarting `nservo`.

### Custom Start Page (`startpage.html`)

When opening a new tab with `:tabnew`, `nservo` checks for `$XDG_CONFIG_HOME/nservo/startpage.html` (falling back to `$HOME/.config/nservo/startpage.html`) and renders it instead of the built-in starter page if present.

### SQLite Persistence (Sessions, Bookmarks & History)

All browser state is persisted in a single SQLite database (WAL mode) under `$XDG_CONFIG_HOME/nservo/nservo.db` (falling back to `$HOME/.config/nservo/nservo.db`):

- **Sessions (`sessions`, `session_tabs`):** Named tab sessions saved with `:w` / `:wq` and restored with `:session`. Session names may contain ASCII letters, digits, `-`, and `_`. If a `default` session exists and no CLI URL is passed, `nservo` restores it automatically on startup.
- **Bookmarks (`bookmarks`):** Deduplicated URLs and titles managed via `b`, `B`, `:bookmark`, `:bookmarks`, and `:bmdel`.
- **History (`history`):** Automatically recorded on navigation (skipping internal `about:` and `data:` pages), browsable and searchable via `gh` or `:history [query]`.

---

## Architecture & Testing

`nservo` separates pure browser logic (`src/core`), Servo integration (`src/engine`), and GTK4 UI (`src/ui`) so all core behavior is deterministically testable without a display server. Every `.rs` file is kept at `<= 300` lines, and both modularity and layer isolation are enforced automatically by the test suite.

See [architecture.md](architecture.md) for the full module map, component diagram, and input flow.

Run the test suite:

```bash
cargo test
```

---

## Inspiration & Acknowledgements

- [qutebrowser](https://qutebrowser.org/) — primary inspiration for keyboard-first modal web browsing, hint overlays, and command workflows.
- [Vim](https://www.vim.org/) / [Neovim](https://neovim.io/) — modal editing philosophy, Ex commands, and statusline/tabline aesthetics.
- [Servo](https://servo.org/) and [`servo-gtk`](https://github.com/nacho/servo-gtk) — the Rust web rendering engine and GTK4 embedding layer powering `nservo`.

---

## License

`nservo` is licensed under the **BSD 3-Clause License**. See [LICENSE](LICENSE) for details.
