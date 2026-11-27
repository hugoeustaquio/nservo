use std::collections::{BTreeSet, HashMap};

use crate::core::{CallFrame, ScriptSource, format_source_with_lines};
use crate::engine::rdp_client::{DebuggerCommand, RdpDebuggerClient};

#[derive(Default)]
pub(super) struct DebuggerState {
    pub connected_tab: Option<u64>,
    pub connected_port: Option<u16>,
    pub client: Option<RdpDebuggerClient>,
    pub sources: Vec<ScriptSource>,
    pub selected_actor: Option<String>,
    pub source_text: HashMap<String, String>,
    pub breakable: HashMap<String, BTreeSet<u32>>,
    pub breakpoints: HashMap<String, BTreeSet<u32>>,
    pub frames: Vec<CallFrame>,
    pub paused_actor: Option<String>,
    pub paused_line: Option<u32>,
}

impl DebuggerState {
    pub fn send(&self, cmd: DebuggerCommand) {
        if let Some(c) = self.client.as_ref() {
            c.send(cmd);
        }
    }

    pub fn toggle_breakpoint(&mut self, line: u32) -> Option<(String, bool)> {
        if line == 0 {
            return None;
        }
        let actor = self.selected_actor.clone()?;
        let url = self
            .sources
            .iter()
            .find(|s| s.actor == actor)
            .map(|s| s.url.clone())?;
        if url.is_empty() {
            return None;
        }
        let set = self.breakpoints.entry(url.clone()).or_default();
        let enable = if set.contains(&line) {
            set.remove(&line);
            false
        } else {
            set.insert(line);
            true
        };
        Some((url, enable))
    }

    pub fn rendered_selected_source(&self) -> Option<String> {
        let actor = self.selected_actor.as_deref()?;
        let content = self
            .source_text
            .get(actor)
            .map(String::as_str)
            .unwrap_or("// Loading source…");
        let url = self
            .sources
            .iter()
            .find(|s| s.actor == actor)
            .map(|s| s.url.as_str())
            .unwrap_or("");
        let bps = self.breakpoints.get(url).cloned().unwrap_or_default();
        let brk = self.breakable.get(actor).cloned().unwrap_or_default();
        let paused = (self.paused_actor.as_deref() == Some(actor))
            .then_some(self.paused_line)
            .flatten();
        Some(format_source_with_lines(content, &bps, &brk, paused))
    }
}
