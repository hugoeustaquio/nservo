use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use crate::core::{
    DebuggerEvent, ScriptSource, StepAction, encode_rdp_packet, try_decode_rdp_packet,
};
use crate::engine::rdp_dispatch::{dispatch_packet, handle_command};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const READ_POLL_TIMEOUT: Duration = Duration::from_millis(50);

/// Commands sent from the GTK Debugger tab to the background RDP client thread.
#[derive(Debug, Clone)]
pub enum DebuggerCommand {
    RefreshSources,
    LoadSource(String),
    ToggleBreakpoint {
        source_url: String,
        line: u32,
        column: u32,
        enable: bool,
    },
    Pause,
    Resume(StepAction),
    LoadScope(String),
    Disconnect,
}

/// Non-blocking Firefox RDP client connected to a tab's Servo DevTools server.
pub struct RdpDebuggerClient {
    cmd_tx: Sender<DebuggerCommand>,
    ev_rx: Receiver<DebuggerEvent>,
}

impl RdpDebuggerClient {
    pub fn connect(port: u16, token: String) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (ev_tx, ev_rx) = mpsc::channel();
        thread::spawn(move || run_session(port, token, cmd_rx, ev_tx));
        Self { cmd_tx, ev_rx }
    }

    pub fn send(&self, cmd: DebuggerCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    pub fn try_recv(&self) -> Option<DebuggerEvent> {
        self.ev_rx.try_recv().ok()
    }
}

impl Drop for RdpDebuggerClient {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(DebuggerCommand::Disconnect);
    }
}

pub(crate) struct SessionActors {
    pub thread_actor: String,
    pub breakpoint_list_actor: String,
    pub sources: Vec<ScriptSource>,
}

pub(crate) fn send_packet(stream: &mut TcpStream, value: &Value) -> std::io::Result<()> {
    let bytes = encode_rdp_packet(value);
    stream.write_all(&bytes)
}

fn run_session(
    port: u16,
    token: String,
    cmd_rx: Receiver<DebuggerCommand>,
    ev_tx: Sender<DebuggerEvent>,
) {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = match TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) {
        Ok(s) => s,
        Err(err) => {
            let _ = ev_tx.send(DebuggerEvent::Disconnected(format!(
                "Could not connect to Servo DevTools on 127.0.0.1:{port}: {err}"
            )));
            return;
        }
    };

    // Bypass Servo's 500ms permission peek immediately when the 8-char auth token matches.
    if token.len() == 8 {
        let auth = format!("25:{{\"auth_token\":\"{token}\"}}");
        if stream.write_all(auth.as_bytes()).is_err() {
            let _ = ev_tx.send(DebuggerEvent::Disconnected("Auth write failed".to_string()));
            return;
        }
    }

    let mut read_buf = Vec::with_capacity(8192);
    let mut actors = match perform_handshake(&mut stream, &mut read_buf) {
        Ok(a) => a,
        Err(err) => {
            let _ = ev_tx.send(DebuggerEvent::Disconnected(err));
            return;
        }
    };

    let _ = ev_tx.send(DebuggerEvent::Connected { port });
    let _ = send_packet(
        &mut stream,
        &json!({ "to": actors.thread_actor, "type": "sources" }),
    );
    let _ = stream.set_read_timeout(Some(READ_POLL_TIMEOUT));

    let mut tmp = [0u8; 8192];
    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            if matches!(cmd, DebuggerCommand::Disconnect) {
                return;
            }
            if handle_command(&mut stream, &actors, cmd).is_err() {
                let _ = ev_tx.send(DebuggerEvent::Disconnected(
                    "Lost connection to Servo DevTools".to_string(),
                ));
                return;
            }
        }

        match stream.read(&mut tmp) {
            Ok(0) => {
                let _ = ev_tx.send(DebuggerEvent::Disconnected("Debugger closed".to_string()));
                return;
            }
            Ok(n) => {
                read_buf.extend_from_slice(&tmp[..n]);
                loop {
                    match try_decode_rdp_packet(&mut read_buf) {
                        Ok(Some(pkt)) => dispatch_packet(&mut stream, &mut actors, &pkt, &ev_tx),
                        Ok(None) => break,
                        Err(err) => {
                            let _ = ev_tx.send(DebuggerEvent::Disconnected(err));
                            return;
                        }
                    }
                }
            }
            Err(err) if matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(err) => {
                let _ = ev_tx.send(DebuggerEvent::Disconnected(format!(
                    "RDP read error: {err}"
                )));
                return;
            }
        }
    }
}

fn perform_handshake(stream: &mut TcpStream, buf: &mut Vec<u8>) -> Result<SessionActors, String> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(4)));
    let _greeting = read_next_packet(stream, buf)?;

    let tabs_reply = request_from(
        stream,
        buf,
        "root",
        &json!({ "to": "root", "type": "listTabs" }),
    )?;
    let tab_actor = tabs_reply
        .get("tabs")
        .and_then(Value::as_array)
        .and_then(|tabs| tabs.first())
        .and_then(|tab| tab.get("actor"))
        .and_then(Value::as_str)
        .ok_or_else(|| "No active tab found in Servo DevTools (wait for page load)".to_string())?
        .to_string();

    let target_reply = request_from(
        stream,
        buf,
        &tab_actor,
        &json!({ "to": tab_actor, "type": "getTarget" }),
    )?;
    let thread_actor = target_reply
        .get("frame")
        .and_then(|f| f.get("threadActor"))
        .and_then(Value::as_str)
        .ok_or_else(|| "Missing threadActor in DevTools getTarget reply".to_string())?
        .to_string();

    let watcher_reply = request_from(
        stream,
        buf,
        &tab_actor,
        &json!({ "to": tab_actor, "type": "getWatcher" }),
    )?;
    let watcher_actor = watcher_reply
        .get("watcher")
        .and_then(|w| w.get("actor"))
        .and_then(Value::as_str)
        .ok_or_else(|| "Missing watcher actor in DevTools getWatcher reply".to_string())?
        .to_string();

    let bp_reply = request_from(
        stream,
        buf,
        &watcher_actor,
        &json!({ "to": watcher_actor, "type": "getBreakpointListActor" }),
    )?;
    let breakpoint_list_actor = bp_reply
        .get("breakpointList")
        .and_then(|b| b.get("actor"))
        .and_then(Value::as_str)
        .ok_or_else(|| "Missing breakpointList actor".to_string())?
        .to_string();

    let _ = send_packet(
        stream,
        &json!({ "to": watcher_actor, "type": "watchTargets", "targetType": "frame" }),
    );
    let _ = send_packet(
        stream,
        &json!({ "to": watcher_actor, "type": "watchResources", "resourceTypes": ["source"] }),
    );
    let _ = send_packet(stream, &json!({ "to": thread_actor, "type": "attach" }));

    Ok(SessionActors {
        thread_actor,
        breakpoint_list_actor,
        sources: Vec::new(),
    })
}

fn request_from(
    stream: &mut TcpStream,
    buf: &mut Vec<u8>,
    expected_from: &str,
    payload: &Value,
) -> Result<Value, String> {
    send_packet(stream, payload).map_err(|e| format!("RDP write failed: {e}"))?;
    for _ in 0..32 {
        let pkt = read_next_packet(stream, buf)?;
        if pkt.get("from").and_then(Value::as_str) == Some(expected_from) {
            return Ok(pkt);
        }
    }
    Err(format!(
        "Timed out waiting for reply from '{expected_from}'"
    ))
}

fn read_next_packet(stream: &mut TcpStream, buf: &mut Vec<u8>) -> Result<Value, String> {
    let mut tmp = [0u8; 8192];
    loop {
        if let Some(pkt) = try_decode_rdp_packet(buf)? {
            return Ok(pkt);
        }
        let n = stream
            .read(&mut tmp)
            .map_err(|e| format!("RDP read failed: {e}"))?;
        if n == 0 {
            return Err("Servo DevTools closed connection during handshake".to_string());
        }
        buf.extend_from_slice(&tmp[..n]);
    }
}
