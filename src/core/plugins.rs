use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Manifest filename inside each `$XDG_CONFIG_HOME/nservo/plugins/<name>/` directory.
pub const PLUGIN_MANIFEST_FILENAME: &str = "plugin.json";

/// Default timeout for `exec` plugin commands in milliseconds.
pub const DEFAULT_EXEC_TIMEOUT_MS: u64 = 10_000;

/// Execution mode declared by a plugin's `plugin.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginMode {
    /// Injects/evaluates local JS & optional CSS directly inside Servo's SpiderMonkey JIT.
    PageJs,
    /// Runs a native executable/script on demand and dispatches JSON-line actions from stdout.
    Exec,
}

/// Declarative `plugin.json` manifest describing an installed plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub default_shortcut: Option<String>,
    pub mode: PluginMode,
    /// Relative JS file path for `page_js` plugins.
    #[serde(default)]
    pub script: Option<String>,
    /// Optional relative CSS file path for `page_js` plugins.
    #[serde(default)]
    pub style: Option<String>,
    /// Executable command path/name for `exec` plugins (relative paths resolve from plugin dir).
    #[serde(default)]
    pub command: Option<String>,
    /// Optional execution timeout in milliseconds for `exec` plugins.
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// A discovered plugin on disk under `$XDG_CONFIG_HOME/nservo/plugins/<dir>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledPlugin {
    pub dir: PathBuf,
    pub manifest: PluginManifest,
}

impl InstalledPlugin {
    pub fn timeout_ms(&self) -> u64 {
        self.manifest
            .timeout_ms
            .unwrap_or(DEFAULT_EXEC_TIMEOUT_MS)
            .clamp(200, 60_000)
    }

    /// Resolve a relative asset path inside this plugin's directory safely (no `..` escape).
    pub fn resolve_asset(&self, rel: &str) -> Result<PathBuf, String> {
        let candidate = Path::new(rel);
        if candidate.is_absolute()
            || candidate
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(format!(
                "Plugin asset path must stay inside plugin dir: '{rel}'"
            ));
        }
        Ok(self.dir.join(candidate))
    }
}

/// Action emitted by an `exec` plugin as one JSON line on `stdout`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginAction {
    EvalJs(String),
    InsertText(String),
    Copy(String),
    Open { url: String, new_tab: bool },
    Command(String),
    Notice(String),
    Error(String),
}

/// Validate a plugin name (safe directory segment: ASCII alphanumeric, `-`, `_`).
pub fn validate_plugin_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 64
        || name.starts_with('.')
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(format!(
            "Invalid plugin name '{name}' (use letters, digits, '-' or '_')"
        ));
    }
    Ok(())
}

/// Expand a GitHub shorthand (`owner/repo`) or Git URL into `(clone_url, dir_name)`.
pub fn normalize_plugin_repo_url(raw: &str) -> Result<(String, String), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_whitespace) {
        return Err("Plugin repository URL must not be empty or contain spaces".to_string());
    }

    let url = if is_github_shorthand(trimmed) {
        format!("https://github.com/{trimmed}.git")
    } else if trimmed.starts_with("https://")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("ssh://")
        || trimmed.starts_with("git@")
        || trimmed.starts_with("file://")
    {
        trimmed.to_string()
    } else {
        return Err(
            "Use 'owner/repo' (GitHub) or a git URL (https://, ssh://, git@, file://)".to_string(),
        );
    };

    let tail = url
        .trim_end_matches('/')
        .rsplit(['/', ':'])
        .next()
        .unwrap_or("");
    let name = tail.strip_suffix(".git").unwrap_or(tail).to_string();
    validate_plugin_name(&name)?;
    Ok((url, name))
}

fn is_github_shorthand(s: &str) -> bool {
    let Some((owner, repo)) = s.split_once('/') else {
        return false;
    };
    !owner.is_empty()
        && !repo.is_empty()
        && !repo.contains('/')
        && !owner.contains(':')
        && owner
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        && repo
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// Load a single plugin manifest from `plugin_dir/plugin.json`.
pub fn load_plugin_from_dir(plugin_dir: &Path) -> Result<InstalledPlugin, String> {
    let manifest_path = plugin_dir.join(PLUGIN_MANIFEST_FILENAME);
    let raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read {}: {e}", manifest_path.display()))?;
    let manifest: PluginManifest = serde_json::from_str(&raw)
        .map_err(|e| format!("Invalid {}: {e}", manifest_path.display()))?;
    validate_plugin_name(&manifest.name)?;
    match manifest.mode {
        PluginMode::PageJs if manifest.script.as_deref().unwrap_or("").trim().is_empty() => {
            return Err(format!(
                "page_js plugin '{}' requires a 'script' field",
                manifest.name
            ));
        }
        PluginMode::Exec if manifest.command.as_deref().unwrap_or("").trim().is_empty() => {
            return Err(format!(
                "exec plugin '{}' requires a 'command' field",
                manifest.name
            ));
        }
        _ => {}
    }
    Ok(InstalledPlugin {
        dir: plugin_dir.to_path_buf(),
        manifest,
    })
}

/// Discover all valid plugins installed inside `plugins_dir`.
pub fn discover_plugins(plugins_dir: &Path) -> Vec<InstalledPlugin> {
    let Ok(entries) = fs::read_dir(plugins_dir) else {
        return Vec::new();
    };
    let mut plugins = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if let Ok(plugin) = load_plugin_from_dir(&path) {
            plugins.push(plugin);
        }
    }
    plugins.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    plugins
}

/// Parse newline-delimited JSON plugin actions emitted on `stdout`.
pub fn parse_plugin_actions(stdout: &str) -> Result<Vec<PluginAction>, String> {
    let mut actions = Vec::new();
    for (idx, raw) in stdout.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let val: Value = serde_json::from_str(line)
            .map_err(|e| format!("Invalid JSON on plugin stdout line {}: {e}", idx + 1))?;
        let op = val
            .get("op")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("Missing 'op' field on plugin stdout line {}", idx + 1))?;
        let action = match op {
            "eval_js" => PluginAction::EvalJs(req_str(&val, "code", idx + 1)?),
            "insert_text" => PluginAction::InsertText(req_str(&val, "text", idx + 1)?),
            "copy" => PluginAction::Copy(req_str(&val, "text", idx + 1)?),
            "open" => PluginAction::Open {
                url: req_str(&val, "url", idx + 1)?,
                new_tab: val.get("new_tab").and_then(Value::as_bool).unwrap_or(false),
            },
            "command" => PluginAction::Command(req_str(&val, "cmd", idx + 1)?),
            "notice" => PluginAction::Notice(req_str(&val, "text", idx + 1)?),
            "error" => PluginAction::Error(req_str(&val, "text", idx + 1)?),
            other => return Err(format!("Unknown plugin op '{other}' on line {}", idx + 1)),
        };
        actions.push(action);
    }
    Ok(actions)
}

fn req_str(val: &Value, key: &str, line: usize) -> Result<String, String> {
    val.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("Missing string field '{key}' on plugin stdout line {line}"))
}

/// Extract `scheme://host[:port]` origin from `url`, or empty string for non-HTTP URLs.
pub fn extract_origin(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return String::new();
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.is_empty() {
        String::new()
    } else {
        format!("{scheme}://{authority}")
    }
}
