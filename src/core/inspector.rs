use std::collections::{HashMap, VecDeque};

use serde::Deserialize;

const MAX_CONSOLE_ENTRIES_PER_TAB: usize = 400;

/// Severity or origin of a line in the built-in JS console.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleLevel {
    Log,
    Info,
    Warn,
    Error,
    Debug,
    Input,
    Result,
}

impl ConsoleLevel {
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Log => "log",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "err",
            Self::Debug => "dbg",
            Self::Input => ">",
            Self::Result => "<-",
        }
    }

    pub fn tag_name(self) -> &'static str {
        match self {
            Self::Log => "console-log",
            Self::Info => "console-info",
            Self::Warn => "console-warn",
            Self::Error => "console-error",
            Self::Debug => "console-debug",
            Self::Input => "console-input",
            Self::Result => "console-result",
        }
    }

    fn parse(raw: &str) -> Self {
        match raw {
            "info" => Self::Info,
            "warn" => Self::Warn,
            "error" => Self::Error,
            "debug" => Self::Debug,
            "input" => Self::Input,
            "result" => Self::Result,
            _ => Self::Log,
        }
    }
}

/// One formatted console message or REPL input/output entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleEntry {
    pub level: ConsoleLevel,
    pub text: String,
}

/// Summary row for a single DOM element in the inspector tree list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomNodeSummary {
    pub index: usize,
    pub depth: usize,
    pub tag: String,
    pub id: String,
    pub classes: String,
    pub text_preview: String,
}

impl DomNodeSummary {
    /// Format a compact indented single-line representation for the DOM list.
    pub fn display_line(&self) -> String {
        let indent = "  ".repeat(self.depth);
        let mut line = format!("{indent}<{}", self.tag);
        if !self.id.is_empty() {
            line.push('#');
            line.push_str(&self.id);
        }
        if !self.classes.is_empty() {
            for cls in self.classes.split_whitespace().take(3) {
                line.push('.');
                line.push_str(cls);
            }
        }
        line.push('>');
        if !self.text_preview.is_empty() {
            line.push(' ');
            line.push_str(&self.text_preview);
        }
        line
    }
}

/// Detailed attributes, geometry, and computed styles for a selected DOM element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementDetails {
    pub selector: String,
    pub rect: String,
    pub attributes: Vec<(String, String)>,
    pub styles: Vec<(String, String)>,
    pub outer_html: String,
}

impl ElementDetails {
    /// Render element details into readable multi-line text for the inspector pane.
    pub fn format_text(&self) -> String {
        let mut out = format!("Element: {}\nBox:     {}\n", self.selector, self.rect);
        out.push_str("\nAttributes:\n");
        if self.attributes.is_empty() {
            out.push_str("  (none)\n");
        } else {
            for (name, value) in &self.attributes {
                out.push_str(&format!("  {name} = \"{value}\"\n"));
            }
        }
        out.push_str("\nComputed Styles:\n");
        for (name, value) in &self.styles {
            out.push_str(&format!("  {name}: {value}\n"));
        }
        if !self.outer_html.is_empty() {
            out.push_str("\nHTML:\n");
            out.push_str(&self.outer_html);
            out.push('\n');
        }
        out
    }
}

/// Events sent from the page's inspector JS bridge to the native GTK inspector UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorEvent {
    Console(ConsoleEntry),
    DomTree(Vec<DomNodeSummary>),
    ElementPicked {
        nodes: Vec<DomNodeSummary>,
        selected_index: usize,
    },
    PickerCancelled,
    ElementDetails(ElementDetails),
}

#[derive(Deserialize)]
struct RawDomNode {
    index: usize,
    depth: usize,
    tag: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    classes: String,
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RawInspectorEvent {
    Console {
        level: String,
        text: String,
    },
    DomTree {
        nodes: Vec<RawDomNode>,
    },
    ElementPicked {
        nodes: Vec<RawDomNode>,
        selected_index: usize,
    },
    PickerCancelled,
    ElementDetails {
        selector: String,
        #[serde(default)]
        rect: String,
        #[serde(default)]
        attributes: Vec<(String, String)>,
        #[serde(default)]
        styles: Vec<(String, String)>,
        #[serde(default)]
        outer_html: String,
    },
}

/// Parse a JSON message emitted by `window.servoGtk.messageHandlers.nservoInspector`.
pub fn parse_inspector_message(body: &str) -> Option<InspectorEvent> {
    let raw: RawInspectorEvent = serde_json::from_str(body).ok()?;
    Some(match raw {
        RawInspectorEvent::Console { level, text } => InspectorEvent::Console(ConsoleEntry {
            level: ConsoleLevel::parse(&level),
            text,
        }),
        RawInspectorEvent::DomTree { nodes } => InspectorEvent::DomTree(convert_dom_nodes(nodes)),
        RawInspectorEvent::ElementPicked {
            nodes,
            selected_index,
        } => InspectorEvent::ElementPicked {
            nodes: convert_dom_nodes(nodes),
            selected_index,
        },
        RawInspectorEvent::PickerCancelled => InspectorEvent::PickerCancelled,
        RawInspectorEvent::ElementDetails {
            selector,
            rect,
            attributes,
            styles,
            outer_html,
        } => InspectorEvent::ElementDetails(ElementDetails {
            selector,
            rect,
            attributes,
            styles,
            outer_html,
        }),
    })
}

fn convert_dom_nodes(nodes: Vec<RawDomNode>) -> Vec<DomNodeSummary> {
    nodes
        .into_iter()
        .map(|n| DomNodeSummary {
            index: n.index,
            depth: n.depth,
            tag: n.tag,
            id: n.id,
            classes: n.classes,
            text_preview: n.text,
        })
        .collect()
}

/// Bounded per-tab console log store so switching tabs preserves each tab's logs.
#[derive(Debug, Default)]
pub struct ConsoleStore {
    logs: HashMap<u64, VecDeque<ConsoleEntry>>,
}

impl ConsoleStore {
    pub fn push(&mut self, tab_id: u64, entry: ConsoleEntry) {
        let queue = self.logs.entry(tab_id).or_default();
        if queue.len() >= MAX_CONSOLE_ENTRIES_PER_TAB {
            queue.pop_front();
        }
        queue.push_back(entry);
    }

    pub fn entries(&self, tab_id: u64) -> Vec<ConsoleEntry> {
        self.logs
            .get(&tab_id)
            .map(|q| q.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn clear(&mut self, tab_id: u64) {
        if let Some(queue) = self.logs.get_mut(&tab_id) {
            queue.clear();
        }
    }

    pub fn remove_tab(&mut self, tab_id: u64) {
        self.logs.remove(&tab_id);
    }
}
