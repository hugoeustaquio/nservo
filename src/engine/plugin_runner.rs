use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::core::{
    InstalledPlugin, PluginAction, discover_plugins, extract_origin, load_plugin_from_dir,
    normalize_plugin_repo_url, parse_plugin_actions,
};

/// Context passed from the active tab into a plugin invocation.
#[derive(Debug, Clone)]
pub struct PluginContext {
    pub url: String,
    pub title: String,
    pub args: String,
    pub config_dir: PathBuf,
}

/// Summary of `:plugin-update [name]`.
#[derive(Debug, Clone)]
pub struct UpdatePluginsSummary {
    pub total: usize,
    pub updated: usize,
    pub failed: Vec<(String, String)>,
}

/// Clone a plugin Git repository (`owner/repo` or URL) into `plugins_dir/<name>`.
pub fn install_plugin(plugins_dir: &Path, repo: &str) -> Result<InstalledPlugin, String> {
    let (clone_url, dir_name) = normalize_plugin_repo_url(repo)?;
    fs::create_dir_all(plugins_dir)
        .map_err(|e| format!("Failed to create {}: {e}", plugins_dir.display()))?;

    let dest = plugins_dir.join(&dir_name);
    if dest.exists() {
        return Err(format!(
            "Plugin directory '{}' already exists (use :plugin-update {dir_name})",
            dest.display()
        ));
    }

    let output = Command::new("git")
        .args(["clone", "--depth=1", &clone_url])
        .arg(&dest)
        .output()
        .map_err(|e| format!("Failed to launch git: {e}"))?;

    if !output.status.success() {
        let _ = fs::remove_dir_all(&dest);
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("git clone exited with {}", output.status)
        } else {
            stderr
        });
    }

    match load_plugin_from_dir(&dest) {
        Ok(plugin) => Ok(plugin),
        Err(err) => {
            let _ = fs::remove_dir_all(&dest);
            Err(err)
        }
    }
}

/// Update one installed plugin (by name) or all Git-backed plugins in `plugins_dir`.
pub fn update_plugins(plugins_dir: &Path, target: Option<&str>) -> UpdatePluginsSummary {
    let installed = discover_plugins(plugins_dir);
    let selected: Vec<InstalledPlugin> = match target {
        Some(name) => installed
            .into_iter()
            .filter(|p| {
                p.manifest.name == name
                    || p.dir.file_name().and_then(|n| n.to_str()) == Some(name)
            })
            .collect(),
        None => installed,
    };

    let total = selected.len();
    let mut updated = 0usize;
    let mut failed = Vec::new();
    for plugin in selected {
        if !plugin.dir.join(".git").exists() {
            failed.push((plugin.manifest.name, "not a git repository".to_string()));
            continue;
        }
        match Command::new("git")
            .arg("-C")
            .arg(&plugin.dir)
            .args(["pull", "--ff-only"])
            .output()
        {
            Ok(out) if out.status.success() => updated += 1,
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                failed.push((
                    plugin.manifest.name,
                    if err.is_empty() {
                        format!("git pull failed ({})", out.status)
                    } else {
                        err
                    },
                ));
            }
            Err(err) => failed.push((plugin.manifest.name, format!("git failed: {err}"))),
        }
    }
    UpdatePluginsSummary {
        total,
        updated,
        failed,
    }
}

/// Remove an installed plugin directory matching `name` from `plugins_dir`.
pub fn remove_plugin(plugins_dir: &Path, name: &str) -> Result<String, String> {
    let installed = discover_plugins(plugins_dir);
    let plugin = installed
        .into_iter()
        .find(|p| {
            p.manifest.name == name
                || p.dir.file_name().and_then(|n| n.to_str()) == Some(name)
        })
        .ok_or_else(|| format!("Plugin '{name}' is not installed"))?;
    fs::remove_dir_all(&plugin.dir)
        .map_err(|e| format!("Failed to remove {}: {e}", plugin.dir.display()))?;
    Ok(plugin.manifest.name)
}

/// Build the self-contained JavaScript payload for a `page_js` plugin (zero subprocesses).
pub fn build_page_js_payload(
    plugin: &InstalledPlugin,
    ctx: &PluginContext,
) -> Result<String, String> {
    let script_rel = plugin
        .manifest
        .script
        .as_deref()
        .ok_or_else(|| format!("Plugin '{}' is missing 'script'", plugin.manifest.name))?;
    let script_path = plugin.resolve_asset(script_rel)?;
    let script_src = fs::read_to_string(&script_path)
        .map_err(|e| format!("Failed to read {}: {e}", script_path.display()))?;

    let css_snippet = if let Some(style_rel) = plugin.manifest.style.as_deref()
        && !style_rel.trim().is_empty()
    {
        let style_path = plugin.resolve_asset(style_rel)?;
        let css = fs::read_to_string(&style_path)
            .map_err(|e| format!("Failed to read {}: {e}", style_path.display()))?;
        let css_json = serde_json::to_string(&css).unwrap_or_else(|_| "\"\"".to_string());
        let id_json = serde_json::to_string(&format!("nservo-plugin-style-{}", plugin.manifest.name))
            .unwrap_or_else(|_| "\"nservo-plugin-style\"".to_string());
        format!(
            "var __sid={id_json};var __st=document.getElementById(__sid);\
             if(!__st){{__st=document.createElement('style');__st.id=__sid;\
             (document.head||document.documentElement).appendChild(__st);}}\
             __st.textContent={css_json};"
        )
    } else {
        String::new()
    };

    let ctx_json = serde_json::to_string(&json!({
        "name": plugin.manifest.name,
        "url": ctx.url,
        "origin": extract_origin(&ctx.url),
        "title": ctx.title,
        "args": ctx.args,
    }))
    .unwrap_or_else(|_| "{}".to_string());

    Ok(format!(
        "(function(){{window.__nservoPlugin={ctx_json};{css_snippet}\n{script_src}\n}})();"
    ))
}

/// Spawn an `exec` plugin on demand with timeout protection and parse its JSON-lines stdout.
pub fn execute_plugin_command(
    plugin: &InstalledPlugin,
    ctx: &PluginContext,
) -> Result<Vec<PluginAction>, String> {
    let cmd_str = plugin
        .manifest
        .command
        .as_deref()
        .ok_or_else(|| format!("Plugin '{}' is missing 'command'", plugin.manifest.name))?;
    let program: PathBuf = if cmd_str.contains('/') {
        plugin.resolve_asset(cmd_str)?
    } else {
        PathBuf::from(cmd_str)
    };

    let origin = extract_origin(&ctx.url);
    let mut child = Command::new(&program)
        .current_dir(&plugin.dir)
        .env("NSERVO_URL", &ctx.url)
        .env("NSERVO_ORIGIN", &origin)
        .env("NSERVO_TITLE", &ctx.title)
        .env("NSERVO_ARGS", &ctx.args)
        .env("NSERVO_PLUGIN_DIR", &plugin.dir)
        .env("NSERVO_CONFIG_DIR", &ctx.config_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start '{}': {e}", program.display()))?;

    let payload = serde_json::to_string(&json!({
        "url": ctx.url,
        "origin": origin,
        "title": ctx.title,
        "args": ctx.args,
    }))
    .unwrap_or_else(|_| "{}".to_string());
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(payload.as_bytes());
        let _ = stdin.write_all(b"\n");
    }

    let deadline = Instant::now() + Duration::from_millis(plugin.timeout_ms());
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "Plugin '{}' timed out after {}ms",
                    plugin.manifest.name,
                    plugin.timeout_ms()
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(15)),
            Err(e) => return Err(format!("Failed waiting on plugin: {e}")),
        }
    };

    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_string(&mut stdout);
    }
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_string(&mut stderr);
    }

    if !status.success() {
        let detail = stderr.trim();
        return Err(if detail.is_empty() {
            format!("Plugin '{}' exited with {status}", plugin.manifest.name)
        } else {
            detail.to_string()
        });
    }
    parse_plugin_actions(&stdout)
}
