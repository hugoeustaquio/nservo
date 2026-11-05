use nservo::browser::dom::DomNode;
use nservo::browser::layout::{LayoutEngine, LayoutNodeType};

#[test]
fn test_html_dom_and_layout_pipeline() {
    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Servo Web Engine</title>
        </head>
        <body>
            <h1>Servo Parallel Architecture</h1>
            <p>Servo is written in Rust and uses WebRender for GPU acceleration.</p>
            <a href="https://servo.org">Visit Servo Website</a>
            <ul>
                <li>High Performance</li>
                <li>Memory Safety</li>
            </ul>
            <pre>cargo build --release</pre>
        </body>
        </html>
    "#;

    let dom = DomNode::parse_html(html);
    assert_eq!(dom.extract_title(), Some("Servo Web Engine".to_string()));

    let (nodes, total_height) = LayoutEngine::compute_layout(&dom, 1280);
    assert!(total_height > 100);

    // Verify Heading
    let h1_node = nodes.iter().find(|n| n.node_type == LayoutNodeType::Heading1).unwrap();
    assert_eq!(h1_node.text, "Servo Parallel Architecture");

    // Verify Link
    let link_node = nodes.iter().find(|n| n.node_type == LayoutNodeType::Link).unwrap();
    assert_eq!(link_node.href, Some("https://servo.org".to_string()));

    // Verify List Items
    let list_items: Vec<_> = nodes.iter().filter(|n| n.node_type == LayoutNodeType::ListItem).collect();
    assert_eq!(list_items.len(), 2);
    assert_eq!(list_items[0].text, "* High Performance");

    // Verify Code Block
    let code_node = nodes.iter().find(|n| n.node_type == LayoutNodeType::CodeBlock).unwrap();
    assert_eq!(code_node.text, "cargo build --release");
}
