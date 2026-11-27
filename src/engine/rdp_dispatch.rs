use std::net::TcpStream;
use std::sync::mpsc::Sender;

use serde_json::{Value, json};

use crate::core::{
    DebuggerEvent, parse_environment_bindings, parse_frames_array, parse_sources_array,
};
use crate::engine::rdp_client::{DebuggerCommand, SessionActors, send_packet};

pub(crate) fn handle_command(
    stream: &mut TcpStream,
    actors: &SessionActors,
    cmd: DebuggerCommand,
) -> std::io::Result<()> {
    match cmd {
        DebuggerCommand::RefreshSources => {
            send_packet(stream, &json!({ "to": actors.thread_actor, "type": "sources" }))
        }
        DebuggerCommand::LoadSource(actor) => {
            send_packet(stream, &json!({ "to": actor, "type": "source" }))?;
            send_packet(stream, &json!({ "to": actor, "type": "getBreakableLines" }))
        }
        DebuggerCommand::ToggleBreakpoint {
            source_url,
            line,
            column,
            enable,
        } => {
            let msg_type = if enable { "setBreakpoint" } else { "removeBreakpoint" };
            send_packet(
                stream,
                &json!({
                    "to": actors.breakpoint_list_actor,
                    "type": msg_type,
                    "location": { "sourceUrl": source_url, "line": line, "column": column }
                }),
            )
        }
        DebuggerCommand::Pause => {
            send_packet(stream, &json!({ "to": actors.thread_actor, "type": "interrupt" }))
        }
        DebuggerCommand::Resume(step) => {
            let payload = match step.rdp_resume_limit() {
                Some(kind) => json!({
                    "to": actors.thread_actor,
                    "type": "resume",
                    "resumeLimit": { "type": kind }
                }),
                None => json!({ "to": actors.thread_actor, "type": "resume" }),
            };
            send_packet(stream, &payload)
        }
        DebuggerCommand::LoadScope(frame_actor) => {
            send_packet(stream, &json!({ "to": frame_actor, "type": "getEnvironment" }))
        }
        DebuggerCommand::Disconnect => Ok(()),
    }
}

pub(crate) fn dispatch_packet(
    stream: &mut TcpStream,
    actors: &mut SessionActors,
    pkt: &Value,
    ev_tx: &Sender<DebuggerEvent>,
) {
    let from = pkt.get("from").and_then(Value::as_str).unwrap_or("");
    let pkt_type = pkt.get("type").and_then(Value::as_str).unwrap_or("");

    match pkt_type {
        "paused" => {
            let reason = pkt
                .get("why")
                .and_then(|w| w.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("paused")
                .to_string();
            if reason == "attached" {
                return;
            }
            let where_obj = pkt.get("frame").and_then(|f| f.get("where"));
            let source_actor = where_obj
                .and_then(|w| w.get("actor"))
                .and_then(Value::as_str)
                .map(str::to_string);
            let line = where_obj
                .and_then(|w| w.get("line"))
                .and_then(Value::as_u64)
                .map(|l| l as u32);
            let _ = ev_tx.send(DebuggerEvent::Paused {
                reason,
                source_actor,
                line,
            });
            let _ = send_packet(
                stream,
                &json!({ "to": actors.thread_actor, "type": "frames", "start": 0, "count": 40 }),
            );
            return;
        }
        "resumed" => {
            let _ = ev_tx.send(DebuggerEvent::Resumed);
            return;
        }
        "resource-available-array" => {
            if let Some(arr) = pkt.get("array").and_then(Value::as_array) {
                let mut added = false;
                for entry in arr {
                    if let Some(pair) = entry.as_array()
                        && pair.first().and_then(Value::as_str) == Some("source")
                        && let Some(items) = pair.get(1).and_then(Value::as_array)
                    {
                        for src in parse_sources_array(items) {
                            if !actors.sources.iter().any(|s| s.actor == src.actor) {
                                actors.sources.push(src);
                                added = true;
                            }
                        }
                    }
                }
                if added {
                    let _ = ev_tx.send(DebuggerEvent::Sources(actors.sources.clone()));
                }
            }
            return;
        }
        _ => {}
    }

    if let Some(items) = pkt.get("sources").and_then(Value::as_array) {
        actors.sources = parse_sources_array(items);
        let _ = ev_tx.send(DebuggerEvent::Sources(actors.sources.clone()));
    } else if let Some(content) = pkt.get("source").and_then(Value::as_str) {
        let _ = ev_tx.send(DebuggerEvent::SourceContent {
            actor: from.to_string(),
            content: content.to_string(),
        });
    } else if let Some(lines) = pkt.get("lines").and_then(Value::as_array) {
        let parsed: Vec<u32> = lines.iter().filter_map(|v| v.as_u64().map(|n| n as u32)).collect();
        let _ = ev_tx.send(DebuggerEvent::BreakableLines {
            actor: from.to_string(),
            lines: parsed,
        });
    } else if let Some(items) = pkt.get("frames").and_then(Value::as_array) {
        let frames = parse_frames_array(items);
        if let Some(first) = frames.first() {
            let _ = send_packet(
                stream,
                &json!({ "to": first.actor, "type": "getEnvironment" }),
            );
        }
        let _ = ev_tx.send(DebuggerEvent::Frames(frames));
    } else if pkt.get("bindings").is_some() || pkt.get("scopeKind").is_some() {
        let _ = ev_tx.send(DebuggerEvent::ScopeBindings {
            frame_actor: from.to_string(),
            bindings: parse_environment_bindings(pkt),
        });
    }
}
