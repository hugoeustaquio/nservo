use nservo::lua::{BrowserSharedState, LuaRuntime};
use std::sync::{Arc, Mutex};

#[test]
fn test_lua_runtime_integration() {
    let state = Arc::new(Mutex::new(BrowserSharedState::new()));
    let runtime = LuaRuntime::new(state.clone()).unwrap();

    let script = r#"
        set("smooth_scroll", false)
        bind("K", "scroll_up_fast")
        nservo.notify("Lua loaded successfully")
        nservo.tabs.new("https://rust-lang.org")
    "#;

    runtime.execute_string(script).unwrap();

    let st = state.lock().unwrap();
    assert_eq!(st.options.smooth_scroll, false);
    assert_eq!(st.keybindings.get_action("K"), Some(&"scroll_up_fast".to_string()));
    assert!(st.notifications.contains(&"Lua loaded successfully".to_string()));
    assert_eq!(st.tab_actions, vec!["new:https://rust-lang.org".to_string()]);
}
