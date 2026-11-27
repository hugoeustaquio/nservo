use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use nservo::core::{
    DebuggerEvent, StepAction, encode_rdp_packet, format_scope_bindings, format_source_with_lines,
    parse_environment_bindings, parse_frames_array, parse_sources_array, try_decode_rdp_packet,
};
use nservo::engine::rdp_client::{DebuggerCommand, RdpDebuggerClient};

#[test]
fn rdp_wire_framing_roundtrips_partial_and_multibyte_packets() {
    let p1 = json!({ "from": "root", "hello": "olá" });
    let p2 = json!({ "from": "thread1", "type": "resumed" });
    let mut wire = encode_rdp_packet(&p1);
    wire.extend_from_slice(&encode_rdp_packet(&p2));

    // Feed only half of the first packet: must wait without error.
    let mut partial = wire[..8].to_vec();
    assert_eq!(try_decode_rdp_packet(&mut partial), Ok(None));

    // Feed the rest: decodes both packets cleanly in order.
    partial.extend_from_slice(&wire[8..]);
    assert_eq!(try_decode_rdp_packet(&mut partial), Ok(Some(p1)));
    assert_eq!(try_decode_rdp_packet(&mut partial), Ok(Some(p2)));
    assert_eq!(try_decode_rdp_packet(&mut partial), Ok(None));
    assert!(partial.is_empty());
}

#[test]
fn parses_servo_sources_frames_and_nested_scopes() {
    let sources = parse_sources_array(&[json!({
        "actor": "source1",
        "url": "https://example.com/static/app.js?v=2",
        "introductionType": "scriptElement"
    })]);
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].display_label(), "app.js (scriptElement)");

    let frames = parse_frames_array(&[json!({
        "actor": "frame1",
        "displayName": "handleClick",
        "where": { "actor": "source1", "line": 42, "column": 8 }
    })]);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].display_line(), "handleClick  @ 42:8");

    let env = json!({
        "type": "function",
        "scopeKind": "function",
        "bindings": {
            "arguments": [ { "ev": { "value": { "type": "object", "class": "MouseEvent" } } } ],
            "variables": {
                "count": { "value": 7 },
                "empty": { "value": { "type": "undefined" } }
            }
        },
        "parent": {
            "type": "declarative",
            "scopeKind": "global",
            "bindings": {
                "variables": {
                    "title": { "value": { "type": "longString", "initial": "Hello" } }
                }
            }
        }
    });
    let bindings = parse_environment_bindings(&env);
    let formatted = format_scope_bindings(&bindings);
    assert!(formatted.contains("ev = [MouseEvent]"));
    assert!(formatted.contains("count = 7"));
    assert!(formatted.contains("empty = undefined"));
    assert!(formatted.contains("title = \"Hello\"…"));
}

#[test]
fn formats_source_gutter_with_breakpoints_and_paused_line() {
    let bps = BTreeSet::from([2u32, 3u32]);
    let breakable = BTreeSet::from([1u32, 2u32, 3u32]);
    let rendered = format_source_with_lines("let a = 1;\nlet b = 2;\nreturn a + b;", &bps, &breakable, Some(3));
    assert!(rendered.contains(" ·   1 │ let a = 1;"));
    assert!(rendered.contains(" ●   2 │ let b = 2;"));
    assert!(rendered.contains("▶●   3 │ return a + b;"));
    assert_eq!(StepAction::StepOver.rdp_resume_limit(), Some("next"));
    assert_eq!(StepAction::StepIn.rdp_resume_limit(), Some("step"));
    assert_eq!(StepAction::StepOut.rdp_resume_limit(), Some("finish"));
    assert_eq!(StepAction::Continue.rdp_resume_limit(), None);
}

#[test]
fn rdp_client_completes_servo_handshake_breakpoints_and_stepping() {
    let Ok(listener) = TcpListener::bind("127.0.0.1:0") else { return };
    let port = listener.local_addr().unwrap().port();

    let server = thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        let mut buf = Vec::new();
        let mut tmp = [0u8; 4096];

        // Read optional 28-byte auth token packet.
        let mut auth = [0u8; 28];
        s.read_exact(&mut auth).unwrap();
        assert_eq!(&auth, b"25:{\"auth_token\":\"ABCDEF12\"}");

        let _ = s.write_all(&encode_rdp_packet(&json!({ "from": "root", "applicationType": "browser" })));

        loop {
            while let Ok(Some(req)) = try_decode_rdp_packet(&mut buf) {
                let to = req.get("to").and_then(|v| v.as_str()).unwrap_or("");
                let ty = req.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let replies: Vec<serde_json::Value> = match (to, ty) {
                    ("root", "listTabs") => vec![json!({ "from": "root", "tabs": [{ "actor": "tab1" }] })],
                    ("tab1", "getTarget") => vec![json!({ "from": "tab1", "frame": { "threadActor": "thread1" } })],
                    ("tab1", "getWatcher") => vec![json!({ "from": "tab1", "watcher": { "actor": "watcher1" } })],
                    ("watcher1", "getBreakpointListActor") => {
                        vec![json!({ "from": "watcher1", "breakpointList": { "actor": "bp1" } })]
                    }
                    ("thread1", "attach") => vec![
                        json!({ "from": "thread1", "type": "paused", "why": { "type": "attached" } }),
                        json!({ "from": "thread1" }),
                    ],
                    ("thread1", "sources") => vec![json!({
                        "from": "thread1",
                        "sources": [{ "actor": "src1", "url": "https://demo.test/app.js", "introductionType": "scriptElement" }]
                    })],
                    ("src1", "source") => vec![json!({ "from": "src1", "source": "function hi() {\n  return 42;\n}" })],
                    ("src1", "getBreakableLines") => vec![json!({ "from": "src1", "lines": [1, 2] })],
                    ("bp1", "setBreakpoint") => vec![json!({ "from": "bp1" })],
                    ("thread1", "interrupt") => vec![
                        json!({ "from": "thread1" }),
                        json!({ "from": "thread1", "type": "paused", "why": { "type": "interrupted" }, "frame": { "where": { "actor": "src1", "line": 2, "column": 2 } } }),
                    ],
                    ("thread1", "frames") => vec![json!({
                        "from": "thread1",
                        "frames": [{ "actor": "frame1", "displayName": "hi", "where": { "actor": "src1", "line": 2, "column": 2 } }]
                    })],
                    ("frame1", "getEnvironment") => vec![json!({
                        "from": "frame1",
                        "scopeKind": "function",
                        "bindings": { "variables": { "x": { "value": 42 } } }
                    })],
                    ("thread1", "resume") => vec![
                        json!({ "from": "thread1", "type": "resumed" }),
                        json!({ "from": "thread1" }),
                    ],
                    _ => vec![json!({ "from": to })],
                };
                for r in replies {
                    if s.write_all(&encode_rdp_packet(&r)).is_err() { return; }
                }
            }
            match s.read(&mut tmp) {
                Ok(0) | Err(_) => break,
                Ok(n) => buf.extend_from_slice(&tmp[..n]),
            }
        }
    });

    let client = RdpDebuggerClient::connect(port, "ABCDEF12".to_string());
    let wait_for = |pred: &dyn Fn(&DebuggerEvent) -> bool| -> DebuggerEvent {
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if let Some(ev) = client.try_recv()
                && pred(&ev)
            {
                return ev;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out waiting for expected DebuggerEvent");
    };

    wait_for(&|e| matches!(e, DebuggerEvent::Connected { .. }));
    let ev = wait_for(&|e| matches!(e, DebuggerEvent::Sources(s) if s.len() == 1));
    assert!(matches!(ev, DebuggerEvent::Sources(_)));

    client.send(DebuggerCommand::LoadSource("src1".to_string()));
    wait_for(&|e| matches!(e, DebuggerEvent::SourceContent { actor, .. } if actor == "src1"));
    wait_for(&|e| matches!(e, DebuggerEvent::BreakableLines { lines, .. } if lines == &[1, 2]));

    client.send(DebuggerCommand::ToggleBreakpoint {
        source_url: "https://demo.test/app.js".to_string(),
        line: 2,
        column: 0,
        enable: true,
    });
    client.send(DebuggerCommand::Pause);
    wait_for(&|e| matches!(e, DebuggerEvent::Paused { line: Some(2), .. }));
    wait_for(&|e| matches!(e, DebuggerEvent::Frames(f) if f.len() == 1));
    wait_for(&|e| matches!(e, DebuggerEvent::ScopeBindings { bindings, .. } if bindings.len() == 1));

    client.send(DebuggerCommand::Resume(StepAction::StepOver));
    wait_for(&|e| matches!(e, DebuggerEvent::Resumed));

    drop(client);
    let _ = server.join();
}
