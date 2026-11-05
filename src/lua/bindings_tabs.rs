use std::sync::{Arc, Mutex};
use mlua::{Lua, Table};
use crate::error::Result;
use crate::lua::state::BrowserSharedState;

pub fn register_nservo_tabs(lua: &Lua, nservo: &Table, state: Arc<Mutex<BrowserSharedState>>) -> Result<()> {
    let tabs_table = lua.create_table()?;

    // nservo.tabs.new(url)
    let s_new = state.clone();
    let new_fn = lua.create_function(move |_lua, url: Option<String>| {
        let target = url.unwrap_or_else(|| "about:blank".to_string());
        if let Ok(mut st) = s_new.lock() {
            st.tab_actions.push(format!("new:{}", target));
        }
        Ok(())
    })?;
    tabs_table.set("new", new_fn)?;

    // nservo.tabs.close(index)
    let s_close = state.clone();
    let close_fn = lua.create_function(move |_lua, idx: Option<usize>| {
        if let Ok(mut st) = s_close.lock() {
            let action = match idx {
                Some(i) => format!("close:{}", i),
                None => "close_current".to_string(),
            };
            st.tab_actions.push(action);
        }
        Ok(())
    })?;
    tabs_table.set("close", close_fn)?;

    // nservo.tabs.next()
    let s_next = state.clone();
    let next_fn = lua.create_function(move |_lua, ()| {
        if let Ok(mut st) = s_next.lock() {
            st.tab_actions.push("next".to_string());
        }
        Ok(())
    })?;
    tabs_table.set("next", next_fn)?;

    // nservo.tabs.prev()
    let s_prev = state.clone();
    let prev_fn = lua.create_function(move |_lua, ()| {
        if let Ok(mut st) = s_prev.lock() {
            st.tab_actions.push("prev".to_string());
        }
        Ok(())
    })?;
    tabs_table.set("prev", prev_fn)?;

    nservo.set("tabs", tabs_table)?;
    Ok(())
}
