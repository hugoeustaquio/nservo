use std::collections::BTreeSet;
use std::fmt::Write as _;

use serde_json::Value;

/// A loaded JavaScript source reported by Servo's SpiderMonkey Debugger (`debugger.js`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptSource {
    pub actor: String,
    pub url: String,
    pub introduction_type: String,
}

impl ScriptSource {
    pub fn display_label(&self) -> String {
        if self.url.is_empty() {
            return format!("[{}] {}", self.introduction_type, self.actor);
        }
        let clean = self.url.split(['?', '#']).next().unwrap_or(&self.url);
        let tail = clean.rsplit('/').find(|s| !s.is_empty()).unwrap_or(clean);
        if self.introduction_type.is_empty() {
            tail.to_string()
        } else {
            format!("{tail} ({})", self.introduction_type)
        }
    }
}

/// A paused JavaScript call-stack frame reported by Servo's `ThreadActor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallFrame {
    pub actor: String,
    pub display_name: String,
    pub source_actor: String,
    pub line: u32,
    pub column: u32,
}

impl CallFrame {
    pub fn display_line(&self) -> String {
        let name = if self.display_name.is_empty() { "(anonymous)" } else { &self.display_name };
        format!("{name}  @ {}:{}", self.line, self.column)
    }
}

/// A variable or argument binding in a paused frame's lexical scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeBinding {
    pub scope: String,
    pub name: String,
    pub value: String,
}

/// Execution-control actions supported by Servo's `ThreadActor::resume`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepAction {
    Continue,
    StepOver,
    StepIn,
    StepOut,
}

impl StepAction {
    pub const fn rdp_resume_limit(self) -> Option<&'static str> {
        match self {
            Self::Continue => None,
            Self::StepOver => Some("next"),
            Self::StepIn => Some("step"),
            Self::StepOut => Some("finish"),
        }
    }
}

/// Events emitted from the localhost Firefox RDP debugger client to the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebuggerEvent {
    Connected { port: u16 },
    Sources(Vec<ScriptSource>),
    SourceContent { actor: String, content: String },
    BreakableLines { actor: String, lines: Vec<u32> },
    Paused { reason: String, source_actor: Option<String>, line: Option<u32> },
    Frames(Vec<CallFrame>),
    ScopeBindings { frame_actor: String, bindings: Vec<ScopeBinding> },
    Resumed,
    Disconnected(String),
}

/// Encode a JSON value using Firefox RDP's `<byte-length>:<json>` wire framing.
pub fn encode_rdp_packet(value: &Value) -> Vec<u8> {
    let json = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    let header = format!("{}:", json.len());
    let mut out = Vec::with_capacity(header.len() + json.len());
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(json.as_bytes());
    out
}

/// Try to decode one complete `<byte-length>:<json>` RDP packet from `buf`.
pub fn try_decode_rdp_packet(buf: &mut Vec<u8>) -> Result<Option<Value>, String> {
    let Some(colon) = buf.iter().position(|&b| b == b':') else {
        if buf.len() > 16 {
            return Err("malformed RDP packet header (missing ':')".to_string());
        }
        return Ok(None);
    };
    let len_str = std::str::from_utf8(&buf[..colon])
        .map_err(|e| format!("invalid UTF-8 in RDP length prefix: {e}"))?;
    let payload_len: usize = len_str
        .trim()
        .parse()
        .map_err(|e| format!("invalid RDP length '{len_str}': {e}"))?;

    let total = colon + 1 + payload_len;
    if buf.len() < total {
        return Ok(None);
    }
    let parsed = serde_json::from_slice::<Value>(&buf[colon + 1..total])
        .map_err(|e| format!("invalid JSON in RDP packet: {e}"))?;
    buf.drain(..total);
    Ok(Some(parsed))
}

/// Parse a `sources` array from `ThreadActor::sources` or `resource-available-array`.
pub fn parse_sources_array(items: &[Value]) -> Vec<ScriptSource> {
    items
        .iter()
        .filter_map(|item| {
            let actor = item.get("actor")?.as_str()?.to_string();
            let url = item.get("url").and_then(Value::as_str).unwrap_or("").to_string();
            let introduction_type = item.get("introductionType").and_then(Value::as_str).unwrap_or("").to_string();
            Some(ScriptSource { actor, url, introduction_type })
        })
        .collect()
}

/// Parse a `frames` array returned by `ThreadActor::frames`.
pub fn parse_frames_array(items: &[Value]) -> Vec<CallFrame> {
    items
        .iter()
        .filter_map(|item| {
            let actor = item.get("actor")?.as_str()?.to_string();
            let display_name = item.get("displayName").and_then(Value::as_str).unwrap_or("").to_string();
            let w = item.get("where");
            let source_actor = w.and_then(|v| v.get("actor")).and_then(Value::as_str).unwrap_or("").to_string();
            let line = w.and_then(|v| v.get("line")).and_then(Value::as_u64).unwrap_or(0) as u32;
            let column = w.and_then(|v| v.get("column")).and_then(Value::as_u64).unwrap_or(0) as u32;
            Some(CallFrame { actor, display_name, source_actor, line, column })
        })
        .collect()
}

/// Extract readable scope bindings from a `FrameActor::getEnvironment` reply.
pub fn parse_environment_bindings(env: &Value) -> Vec<ScopeBinding> {
    let mut out = Vec::new();
    collect_env(env, 0, &mut out);
    out
}

fn collect_env(env: &Value, depth: usize, out: &mut Vec<ScopeBinding>) {
    if depth > 6 || !env.is_object() {
        return;
    }
    let scope = env
        .get("scopeKind")
        .and_then(Value::as_str)
        .or_else(|| env.get("type").and_then(Value::as_str))
        .unwrap_or("scope")
        .to_string();

    if let Some(bindings) = env.get("bindings") {
        if let Some(args) = bindings.get("arguments").and_then(Value::as_array) {
            for arg in args {
                if let Some(map) = arg.as_object() {
                    for (name, desc) in map {
                        out.push(ScopeBinding {
                            scope: format!("{scope} (arg)"),
                            name: name.clone(),
                            value: format_rdp_value(desc.get("value").unwrap_or(desc)),
                        });
                    }
                }
            }
        }
        if let Some(vars) = bindings.get("variables").and_then(Value::as_object) {
            for (name, desc) in vars {
                out.push(ScopeBinding {
                    scope: scope.clone(),
                    name: name.clone(),
                    value: format_rdp_value(desc.get("value").unwrap_or(desc)),
                });
            }
        }
    }
    if let Some(parent) = env.get("parent")
        && !parent.is_null()
    {
        collect_env(parent, depth + 1, out);
    }
}

fn format_rdp_value(val: &Value) -> String {
    match val {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("{s:?}"),
        Value::Array(arr) => format!("Array({})", arr.len()),
        Value::Object(map) => {
            if let Some(ty) = map.get("type").and_then(Value::as_str) {
                return match ty {
                    "undefined" | "null" | "Infinity" | "-Infinity" | "NaN" | "-0" => ty.to_string(),
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
