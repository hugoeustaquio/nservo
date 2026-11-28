use std::collections::BTreeSet;
use std::fmt::Write as _;

use serde_json::Value;

use crate::core::debugger::ScopeBinding;

pub(crate) fn format_rdp_value(val: &Value) -> String {
    match val {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("{s:?}"),
        Value::Array(arr) => format!("Array({})", arr.len()),
        Value::Object(map) => {
            if let Some(ty) = map.get("type").and_then(Value::as_str) {
                return match ty {
                    "undefined" | "null" | "Infinity" | "-Infinity" | "NaN" | "-0" => {
                        ty.to_string()
                    }
                    "longString" => {
                        let initial = map.get("initial").and_then(Value::as_str).unwrap_or("");
                        format!("{initial:?}…")
                    }
                    "object" => {
                        let cls = map.get("class").and_then(Value::as_str).unwrap_or("Object");
                        format!("[{cls}]")
                    }
                    other => format!("<{other}>"),
                };
            }
            val.to_string()
        }
    }
}

/// Render JavaScript source text with 1-based line numbers, breakpoint (`●`),
/// breakable (`·`), and paused-line (`▶`) gutter markers.
pub fn format_source_with_lines(
    content: &str,
    breakpoints: &BTreeSet<u32>,
    breakable: &BTreeSet<u32>,
    paused_line: Option<u32>,
) -> String {
    if content.is_empty() {
        return "// (empty source)".to_string();
    }
    let mut out = String::with_capacity(content.len() + 256);
    for (idx, line) in content.lines().enumerate() {
        let num = (idx + 1) as u32;
        let marker = match (paused_line == Some(num), breakpoints.contains(&num)) {
            (true, true) => "▶●",
            (true, false) => "▶ ",
            (false, true) => " ●",
            (false, false) if breakable.contains(&num) => " ·",
            (false, false) => "  ",
        };
        let _ = writeln!(&mut out, "{marker}{num:>4} │ {line}");
    }
    out
}

/// Format scope bindings for the debugger Scopes text pane.
pub fn format_scope_bindings(bindings: &[ScopeBinding]) -> String {
    if bindings.is_empty() {
        return "No local variables in selected frame.".to_string();
    }
    let mut out = String::new();
    let mut last_scope = "";
    for b in bindings {
        if b.scope != last_scope {
            if !out.is_empty() {
                out.push('\n');
            }
            let _ = writeln!(&mut out, "[{}]", b.scope);
            last_scope = &b.scope;
        }
        let _ = writeln!(&mut out, "  {} = {}", b.name, b.value);
    }
    out
}
