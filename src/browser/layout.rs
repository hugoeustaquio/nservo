use crate::browser::dom::{DomNode, DomNodeType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutNodeType {
    Heading1,
    Heading2,
    Heading3,
    Paragraph,
    Link,
    ListItem,
    CodeBlock,
    InputField,
    Button,
}

#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub node_type: LayoutNodeType,
    pub text: String,
    pub href: Option<String>,
    pub fg_color: u32,
    pub bg_color: Option<u32>,
}

pub struct LayoutEngine;

impl LayoutEngine {
    pub fn compute_layout(dom: &DomNode, viewport_w: usize) -> (Vec<LayoutNode>, usize) {
        let mut nodes = Vec::new();
        let mut cur_y = 24;
        let margin_x = 48;
        let content_w = viewport_w.saturating_sub(margin_x * 2).max(300);

        let mut current_tag = "p".to_string();
        let mut current_href = None;

        for child in &dom.children {
            match &child.node_type {
                DomNodeType::Element { tag, href } => {
                    current_tag = tag.clone();
                    current_href = href.clone();
                }
                DomNodeType::Text(text) => {
                    if text.is_empty() {
                        continue;
                    }

                    match current_tag.as_str() {
                        "h1" => {
                            cur_y += 12;
                            nodes.push(LayoutNode {
                                x: margin_x,
                                y: cur_y,
                                width: content_w,
                                height: 24,
                                node_type: LayoutNodeType::Heading1,
                                text: text.clone(),
                                href: None,
                                fg_color: 0x0038bdf8, // Sky blue
                                bg_color: None,
                            });
                            cur_y += 32;
                        }
                        "h2" | "h3" => {
                            cur_y += 8;
                            nodes.push(LayoutNode {
                                x: margin_x,
                                y: cur_y,
                                width: content_w,
                                height: 20,
                                node_type: LayoutNodeType::Heading2,
                                text: text.clone(),
                                href: None,
                                fg_color: 0x0060a5fa, // Light blue
                                bg_color: None,
                            });
                            cur_y += 26;
                        }
                        "a" => {
                            nodes.push(LayoutNode {
                                x: margin_x + 8,
                                y: cur_y,
                                width: content_w,
                                height: 18,
                                node_type: LayoutNodeType::Link,
                                text: format!("[Link] {}", text),
                                href: current_href.clone(),
                                fg_color: 0x0038bdf8,
                                bg_color: None,
                            });
                            cur_y += 20;
                        }
                        "li" => {
                            nodes.push(LayoutNode {
                                x: margin_x + 16,
                                y: cur_y,
                                width: content_w,
                                height: 18,
                                node_type: LayoutNodeType::ListItem,
                                text: format!("* {}", text),
                                href: None,
                                fg_color: 0x00e4e4e7,
                                bg_color: None,
                            });
                            cur_y += 20;
                        }
                        "pre" | "code" => {
                            nodes.push(LayoutNode {
                                x: margin_x,
                                y: cur_y,
                                width: content_w,
                                height: 24,
                                node_type: LayoutNodeType::CodeBlock,
                                text: text.clone(),
                                href: None,
                                fg_color: 0x00a7f3d0, // Emerald
                                bg_color: Some(0x0027272a),
                            });
                            cur_y += 28;
                        }
                        _ => {
                            // Standard paragraph with line wrapping
                            let max_line_chars = content_w / 8;
                            let lines = Self::wrap_text(text, max_line_chars.max(20));
                            for line in lines {
                                nodes.push(LayoutNode {
                                    x: margin_x,
                                    y: cur_y,
                                    width: content_w,
                                    height: 18,
                                    node_type: LayoutNodeType::Paragraph,
                                    text: line,
                                    href: None,
                                    fg_color: 0x00d4d4d8,
                                    bg_color: None,
                                });
                                cur_y += 18;
                            }
                            cur_y += 6;
                        }
                    }
                }
                _ => {}
            }
        }

        (nodes, cur_y + 40)
    }

    fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            if current_line.len() + word.len() + 1 > max_chars {
                if !current_line.is_empty() {
                    lines.push(current_line);
                    current_line = String::new();
                }
            }
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }
}
