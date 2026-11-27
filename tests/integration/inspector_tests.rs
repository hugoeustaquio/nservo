use nservo::core::{
    ConsoleEntry, ConsoleLevel, ConsoleStore, DomNodeSummary, ElementDetails, InspectorEvent,
    parse_inspector_message,
};
use nservo::engine::inspector_js::{
    INSPECTOR_MESSAGE_HANDLER, clear_highlight_script, eval_in_page_script, inspect_element_script,
    inspector_init_script, snapshot_dom_script,
};

#[test]
fn parses_console_dom_and_element_details_messages() {
    let console = parse_inspector_message(r#"{"kind":"console","level":"warn","text":"careful"}"#);
    assert_eq!(
        console,
        Some(InspectorEvent::Console(ConsoleEntry {
            level: ConsoleLevel::Warn,
            text: "careful".to_string(),
        }))
    );

    let dom = parse_inspector_message(
        r#"{"kind":"dom_tree","nodes":[{"index":0,"depth":0,"tag":"html"},{"index":1,"depth":1,"tag":"div","id":"app","classes":"main hero","text":"hello"}]}"#,
    );
    assert_eq!(
        dom,
        Some(InspectorEvent::DomTree(vec![
            DomNodeSummary {
                index: 0,
                depth: 0,
                tag: "html".to_string(),
                id: String::new(),
                classes: String::new(),
                text_preview: String::new(),
            },
            DomNodeSummary {
                index: 1,
                depth: 1,
                tag: "div".to_string(),
                id: "app".to_string(),
                classes: "main hero".to_string(),
                text_preview: "hello".to_string(),
            },
        ]))
    );

    let details = parse_inspector_message(
        r#"{"kind":"element_details","selector":"div#app","rect":"100x50 @ (0, 0)","attributes":[["id","app"]],"styles":[["display","block"]],"outer_html":"<div id=\"app\"></div>"}"#,
    );
    assert_eq!(
        details,
        Some(InspectorEvent::ElementDetails(ElementDetails {
            selector: "div#app".to_string(),
            rect: "100x50 @ (0, 0)".to_string(),
            attributes: vec![("id".to_string(), "app".to_string())],
            styles: vec![("display".to_string(), "block".to_string())],
            outer_html: "<div id=\"app\"></div>".to_string(),
        }))
    );

    assert_eq!(parse_inspector_message("not json"), None);
}

#[test]
fn dom_summary_and_details_format_cleanly() {
    let node = DomNodeSummary {
        index: 2,
        depth: 2,
        tag: "button".to_string(),
        id: "save".to_string(),
        classes: "btn primary".to_string(),
        text_preview: "Save".to_string(),
    };
    assert_eq!(node.display_line(), "    <button#save.btn.primary> Save");

    let details = ElementDetails {
        selector: "button#save".to_string(),
        rect: "80x32 @ (10, 20)".to_string(),
        attributes: vec![("id".to_string(), "save".to_string())],
        styles: vec![("display".to_string(), "inline-block".to_string())],
        outer_html: "<button id=\"save\">Save</button>".to_string(),
    };
    let text = details.format_text();
    assert!(text.contains("Element: button#save"));
    assert!(text.contains("id = \"save\""));
    assert!(text.contains("display: inline-block"));
}

#[test]
fn console_store_isolates_tabs_and_clears() {
    let mut store = ConsoleStore::default();
    store.push(
        1,
        ConsoleEntry {
            level: ConsoleLevel::Log,
            text: "tab one".to_string(),
        },
    );
    store.push(
        2,
        ConsoleEntry {
            level: ConsoleLevel::Error,
            text: "tab two".to_string(),
        },
    );
    assert_eq!(store.entries(1).len(), 1);
    assert_eq!(store.entries(2).len(), 1);

    store.clear(1);
    assert!(store.entries(1).is_empty());
    assert_eq!(store.entries(2).len(), 1);
}

#[test]
fn inspector_scripts_protect_servo_ipc_and_encode_expressions() {
    let init = inspector_init_script();
    assert!(init.contains(INSPECTOR_MESSAGE_HANDLER));
    assert!(init.contains("__servo_gtk_msg__"));
    assert!(init.contains("unhandledrejection"));

    let eval_js = eval_in_page_script("1 + \"ação\"");
    assert!(eval_js.contains(r#""1 + \"ação\"""#));
    assert!(eval_js.contains(INSPECTOR_MESSAGE_HANDLER));

    assert!(snapshot_dom_script().contains("dom_tree"));
    assert!(inspect_element_script(3).contains("refs[3]"));
    assert!(clear_highlight_script().contains("nservo-inspector-highlight"));
}
